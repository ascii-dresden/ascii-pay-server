//! Inventory persistence: stock overview rows and inventory checks.

use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::{Connection, FromRow, PgConnection, Row};

use crate::error::{ServiceError, ServiceResult};
use crate::models;

use super::purchases::{fetch_purchase, insert_purchase, insert_purchase_item};
use super::{
    to_service_result, DatabaseConnection, InventoryCheckStateDto, ProductRow, PRODUCT_COLUMNS,
    PRODUCT_STATUS_JOINS,
};

const CHECK_COMPLETED: &str = "Inventory check is completed";
const DRAFT_CHECK_EXISTS: &str = "A draft inventory check already exists";

/// Columns mapping into `InventoryCheckRow`; expects `inventory_check` aliased as `ic`.
const INVENTORY_CHECK_COLUMNS: &str = r#"
    ic.id, ic.state, ic.started_at, ic.completed_at, ic.started_by_account_id,
    ic.completed_by_account_id, ic.generated_purchase_id, ic.note,
    (SELECT count(*) FROM inventory_check_item i WHERE i.check_id = ic.id) AS item_count,
    (SELECT count(*) FROM inventory_check_item i WHERE i.check_id = ic.id AND i.counted_quantity IS NOT NULL) AS counted_count
"#;

#[derive(sqlx::FromRow)]
struct InventoryCheckRow {
    #[sqlx(try_from = "i64")]
    id: u64,
    state: InventoryCheckStateDto,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    started_by_account_id: Option<i64>,
    completed_by_account_id: Option<i64>,
    generated_purchase_id: Option<i64>,
    note: String,
    item_count: i64,
    counted_count: i64,
}

impl From<InventoryCheckRow> for models::InventoryCheck {
    fn from(value: InventoryCheckRow) -> Self {
        models::InventoryCheck {
            id: value.id,
            state: value.state.into(),
            started_at: value.started_at,
            completed_at: value.completed_at,
            started_by_account_id: value.started_by_account_id.map(to_u64),
            completed_by_account_id: value.completed_by_account_id.map(to_u64),
            generated_purchase_id: value.generated_purchase_id.map(to_u64),
            note: value.note,
            item_count: to_u64(value.item_count),
            counted_count: to_u64(value.counted_count),
            items: Vec::new(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct InventoryCheckItemRow {
    check_item_target_quantity: i32,
    check_item_counted_quantity: Option<i32>,
}

#[derive(sqlx::FromRow)]
struct InventoryStatsRow {
    product_id: i64,
    last_counted_quantity: Option<i32>,
    last_counted_at: Option<DateTime<Utc>>,
    estimated_quantity: Option<i64>,
    last_purchase_id: Option<i64>,
    last_purchase_timestamp: Option<DateTime<Utc>>,
    last_purchase_container_size: Option<i32>,
    last_purchase_container_cents: Option<i32>,
}

#[derive(sqlx::FromRow)]
struct GeneratedItemRow {
    product_id: i64,
    name: String,
    missing: i32,
    last_container_size: Option<i32>,
    last_container_cents: Option<i32>,
}

fn to_u64(id: i64) -> u64 {
    id.try_into().expect("IDs are non-negative")
}

fn to_i64(id: u64) -> i64 {
    i64::try_from(id).expect("id is always less than 2**63")
}

fn check_item_query(where_clause: &str) -> String {
    format!(
        r#"
        SELECT
            ici.target_quantity AS check_item_target_quantity,
            ici.counted_quantity AS check_item_counted_quantity,
            {PRODUCT_COLUMNS}
        FROM inventory_check_item ici
            INNER JOIN product p ON p.id = ici.product_id
            {PRODUCT_STATUS_JOINS}
        WHERE {where_clause}
        GROUP BY ici.check_id, ici.product_id, p.id
        ORDER BY p.category ASC, p.name ASC, p.id ASC
        "#
    )
}

fn check_item_from_row(row: &PgRow) -> ServiceResult<models::InventoryCheckItem> {
    let item_row = to_service_result(InventoryCheckItemRow::from_row(row))?;
    let product: models::Product = to_service_result(ProductRow::from_row(row))?.into();
    Ok(models::InventoryCheckItem {
        product,
        target_quantity: item_row.check_item_target_quantity,
        counted_quantity: item_row.check_item_counted_quantity,
    })
}

async fn fetch_check(
    conn: &mut PgConnection,
    id: u64,
) -> ServiceResult<Option<models::InventoryCheck>> {
    let r = sqlx::query_as::<_, InventoryCheckRow>(&format!(
        "SELECT {INVENTORY_CHECK_COLUMNS} FROM inventory_check ic WHERE ic.id = $1"
    ))
    .bind(to_i64(id))
    .fetch_optional(&mut *conn)
    .await;

    let Some(row) = to_service_result(r)? else {
        return Ok(None);
    };
    let mut check = models::InventoryCheck::from(row);

    let rows = sqlx::query(&check_item_query("ici.check_id = $1"))
        .bind(to_i64(id))
        .fetch_all(conn)
        .await;
    for row in to_service_result(rows)? {
        check.items.push(check_item_from_row(&row)?);
    }

    Ok(Some(check))
}

async fn fetch_check_item(
    conn: &mut PgConnection,
    check_id: u64,
    product_id: u64,
) -> ServiceResult<Option<models::InventoryCheckItem>> {
    let r = sqlx::query(&check_item_query(
        "ici.check_id = $1 AND ici.product_id = $2",
    ))
    .bind(to_i64(check_id))
    .bind(to_i64(product_id))
    .fetch_optional(conn)
    .await;

    match to_service_result(r)? {
        Some(row) => Ok(Some(check_item_from_row(&row)?)),
        None => Ok(None),
    }
}

/// Locks the check row for the rest of the transaction and returns its state.
async fn lock_check(conn: &mut PgConnection, id: u64) -> ServiceResult<InventoryCheckStateDto> {
    let r = sqlx::query("SELECT state FROM inventory_check WHERE id = $1 FOR UPDATE")
        .bind(to_i64(id))
        .fetch_optional(conn)
        .await;

    match to_service_result(r)? {
        Some(row) => Ok(row.get("state")),
        None => Err(ServiceError::NotFound),
    }
}

async fn lock_draft_check(conn: &mut PgConnection, id: u64) -> ServiceResult<()> {
    match lock_check(conn, id).await? {
        InventoryCheckStateDto::Draft => Ok(()),
        InventoryCheckStateDto::Completed => Err(ServiceError::Conflict(CHECK_COMPLETED)),
    }
}

/// Number of containers needed to cover `missing` units, rounded up.
fn containers_needed(missing: i32, container_size: i32) -> i32 {
    let size = container_size.max(1);
    (missing + size - 1) / size
}

impl DatabaseConnection {
    /// One row per stocked product with the last count, estimate and last purchase.
    pub async fn get_inventory(&mut self) -> ServiceResult<Vec<models::InventoryRow>> {
        let products = sqlx::query_as::<_, ProductRow>(&format!(
            r#"
            SELECT {PRODUCT_COLUMNS}
            FROM product p
                {PRODUCT_STATUS_JOINS}
            WHERE p.stocked
            GROUP BY p.id
            ORDER BY p.category ASC, p.name ASC, p.id ASC
            "#
        ))
        .fetch_all(self.connection.as_mut())
        .await;
        let products = to_service_result(products)?;

        let stats = sqlx::query_as::<_, InventoryStatsRow>(
            r#"
            SELECT
                p.id AS product_id,
                lc.counted_quantity AS last_counted_quantity,
                lc.completed_at AS last_counted_at,
                CASE
                    WHEN lc.counted_quantity IS NULL THEN NULL
                    ELSE lc.counted_quantity::BIGINT + coalesce(pur.units, 0) - coalesce(ti.cnt, 0)
                END AS estimated_quantity,
                lp.purchase_id AS last_purchase_id,
                lp.timestamp AS last_purchase_timestamp,
                lp.container_size AS last_purchase_container_size,
                lp.container_cents AS last_purchase_container_cents
            FROM product p
                LEFT JOIN LATERAL (
                    SELECT ici.counted_quantity, ic.completed_at
                    FROM inventory_check_item ici
                        INNER JOIN inventory_check ic ON ic.id = ici.check_id
                    WHERE ici.product_id = p.id
                        AND ic.state = 'completed'
                        AND ici.counted_quantity IS NOT NULL
                    ORDER BY ic.completed_at DESC, ic.id DESC
                    LIMIT 1
                ) lc ON TRUE
                LEFT JOIN LATERAL (
                    SELECT sum(pi.container_count::BIGINT * pi.container_size::BIGINT)::BIGINT AS units
                    FROM purchase_item pi
                        INNER JOIN purchase pu ON pu.id = pi.purchase_id
                    WHERE pi.product_id = p.id
                        AND pu.state = 'finalized'
                        AND pu.timestamp > lc.completed_at
                ) pur ON TRUE
                LEFT JOIN LATERAL (
                    SELECT count(*)::BIGINT AS cnt
                    FROM transaction_item t
                    WHERE t.product_id = p.id AND t.timestamp > lc.completed_at
                ) ti ON TRUE
                LEFT JOIN LATERAL (
                    SELECT pu.id AS purchase_id, pu.timestamp, pi.container_size, pi.container_cents
                    FROM purchase_item pi
                        INNER JOIN purchase pu ON pu.id = pi.purchase_id
                    WHERE pi.product_id = p.id AND pu.state = 'finalized'
                    ORDER BY pu.timestamp DESC, pu.id DESC, pi.created_at DESC, pi.id DESC
                    LIMIT 1
                ) lp ON TRUE
            WHERE p.stocked
            "#,
        )
        .fetch_all(self.connection.as_mut())
        .await;
        let stats = to_service_result(stats)?;

        let mut out = Vec::with_capacity(products.len());
        for product_row in products {
            let product: models::Product = product_row.into();
            let stat = stats.iter().find(|s| to_u64(s.product_id) == product.id);

            let mut row = models::InventoryRow {
                target_quantity: product.target_quantity,
                product,
                last_counted_quantity: None,
                last_counted_at: None,
                estimated_quantity: None,
                last_purchase: None,
            };

            if let Some(stat) = stat {
                row.last_counted_quantity = stat.last_counted_quantity;
                row.last_counted_at = stat.last_counted_at;
                row.estimated_quantity = stat.estimated_quantity;
                if let (Some(purchase_id), Some(timestamp), Some(container_size), Some(cents)) = (
                    stat.last_purchase_id,
                    stat.last_purchase_timestamp,
                    stat.last_purchase_container_size,
                    stat.last_purchase_container_cents,
                ) {
                    row.last_purchase = Some(models::InventoryLastPurchase {
                        purchase_id: to_u64(purchase_id),
                        timestamp,
                        container_size,
                        container_cents: cents,
                    });
                }
            }

            out.push(row);
        }

        Ok(out)
    }

    /// Lists inventory checks (without items), newest first, optionally filtered by state.
    pub async fn get_inventory_checks(
        &mut self,
        state: Option<models::InventoryCheckState>,
    ) -> ServiceResult<Vec<models::InventoryCheck>> {
        let r = sqlx::query_as::<_, InventoryCheckRow>(&format!(
            r#"
            SELECT {INVENTORY_CHECK_COLUMNS}
            FROM inventory_check ic
            WHERE $1::tp_inventory_check_state IS NULL OR ic.state = $1
            ORDER BY ic.started_at DESC, ic.id DESC
            "#
        ))
        .bind(state.map(InventoryCheckStateDto::from))
        .fetch_all(self.connection.as_mut())
        .await;

        Ok(to_service_result(r)?.into_iter().map(Into::into).collect())
    }

    pub async fn get_inventory_check_by_id(
        &mut self,
        id: u64,
    ) -> ServiceResult<Option<models::InventoryCheck>> {
        fetch_check(self.connection.as_mut(), id).await
    }

    /// Creates a draft check with one item per stocked product (target snapshot).
    ///
    /// Fails with `Conflict` if a draft check already exists.
    pub async fn create_inventory_check(
        &mut self,
        note: &str,
        started_by_account_id: Option<u64>,
    ) -> ServiceResult<models::InventoryCheck> {
        let mut tx = self.connection.begin().await?;

        // Serialize concurrent creations so the "single draft" rule holds.
        let r = sqlx::query("LOCK TABLE inventory_check IN SHARE ROW EXCLUSIVE MODE")
            .execute(tx.as_mut())
            .await;
        to_service_result(r)?;

        let r = sqlx::query("SELECT 1 FROM inventory_check WHERE state = 'draft' LIMIT 1")
            .fetch_optional(tx.as_mut())
            .await;
        if to_service_result(r)?.is_some() {
            return Err(ServiceError::Conflict(DRAFT_CHECK_EXISTS));
        }

        let r = sqlx::query(
            r#"
            INSERT INTO inventory_check (note, started_by_account_id)
            VALUES ($1, $2)
            RETURNING id
            "#,
        )
        .bind(note)
        .bind(started_by_account_id.map(to_i64))
        .fetch_one(tx.as_mut())
        .await;
        let check_id = to_service_result(r)?.get::<i64, _>(0);

        let r = sqlx::query(
            r#"
            INSERT INTO inventory_check_item (check_id, product_id, target_quantity)
            SELECT $1, id, target_quantity FROM product WHERE stocked
            "#,
        )
        .bind(check_id)
        .execute(tx.as_mut())
        .await;
        to_service_result(r)?;

        to_service_result(tx.commit().await)?;

        fetch_check(self.connection.as_mut(), to_u64(check_id))
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Records the counted quantity (or clears it with `None`) for a product of a draft check.
    pub async fn set_inventory_check_item_count(
        &mut self,
        check_id: u64,
        product_id: u64,
        counted_quantity: Option<i32>,
    ) -> ServiceResult<models::InventoryCheckItem> {
        let mut tx = self.connection.begin().await?;

        lock_draft_check(tx.as_mut(), check_id).await?;

        let r = sqlx::query(
            r#"
            UPDATE inventory_check_item
            SET counted_quantity = $3
            WHERE check_id = $1 AND product_id = $2
            "#,
        )
        .bind(to_i64(check_id))
        .bind(to_i64(product_id))
        .bind(counted_quantity)
        .execute(tx.as_mut())
        .await;
        if to_service_result(r)?.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }

        to_service_result(tx.commit().await)?;

        fetch_check_item(self.connection.as_mut(), check_id, product_id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    pub async fn delete_inventory_check(&mut self, id: u64) -> ServiceResult<()> {
        let r = sqlx::query(r#"DELETE FROM inventory_check WHERE id = $1"#)
            .bind(to_i64(id))
            .execute(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;
        if r.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    /// Completes a draft check and optionally generates a draft purchase for all counted
    /// products below their target. Returns the check and the generated purchase (if any).
    ///
    /// Fails with `Conflict` if the check is already completed.
    pub async fn complete_inventory_check(
        &mut self,
        id: u64,
        completion: models::InventoryCheckCompletion,
    ) -> ServiceResult<(models::InventoryCheck, Option<models::Purchase>)> {
        let mut tx = self.connection.begin().await?;

        lock_draft_check(tx.as_mut(), id).await?;

        let mut purchase_id: Option<u64> = None;
        if completion.generate_purchase {
            let r = sqlx::query_as::<_, GeneratedItemRow>(
                r#"
                SELECT
                    ici.product_id,
                    p.name,
                    (ici.target_quantity - ici.counted_quantity) AS missing,
                    lp.container_size AS last_container_size,
                    lp.container_cents AS last_container_cents
                FROM inventory_check_item ici
                    INNER JOIN product p ON p.id = ici.product_id
                    LEFT JOIN LATERAL (
                        SELECT pi.container_size, pi.container_cents
                        FROM purchase_item pi
                            INNER JOIN purchase pu ON pu.id = pi.purchase_id
                        WHERE pi.product_id = ici.product_id AND pu.state = 'finalized'
                        ORDER BY pu.timestamp DESC, pu.id DESC, pi.created_at DESC, pi.id DESC
                        LIMIT 1
                    ) lp ON TRUE
                WHERE ici.check_id = $1
                    AND ici.counted_quantity IS NOT NULL
                    AND ici.target_quantity - ici.counted_quantity > 0
                ORDER BY p.category ASC, p.name ASC, p.id ASC
                "#,
            )
            .bind(to_i64(id))
            .fetch_all(tx.as_mut())
            .await;
            let generated = to_service_result(r)?;

            if !generated.is_empty() {
                let purchase = models::Purchase {
                    id: 0,
                    name: completion.purchase_name,
                    store: completion.purchase_store,
                    timestamp: Utc::now(),
                    state: models::PurchaseState::Draft,
                    purchased_by_account_id: completion.completed_by_account_id,
                    finalized_at: None,
                    finalized_by_account_id: None,
                    items: Vec::new(),
                };
                let new_purchase_id = insert_purchase(tx.as_mut(), &purchase).await?;

                for row in generated {
                    let container_size = row.last_container_size.unwrap_or(1).max(1);
                    let product = models::Product {
                        id: to_u64(row.product_id),
                        // Only the id is used when storing the item.
                        name: String::new(),
                        price: models::CoinAmount::zero(),
                        bonus: models::CoinAmount::zero(),
                        purchase_tax: 0,
                        nickname: None,
                        image: None,
                        barcode: None,
                        category: String::new(),
                        print_lists: Vec::new(),
                        tags: Vec::new(),
                        status_prices: Vec::new(),
                        stocked: false,
                        target_quantity: 0,
                    };
                    let item = models::PurchaseItem {
                        id: 0,
                        name: row.name,
                        container_size,
                        container_count: containers_needed(row.missing, container_size),
                        container_cents: row.last_container_cents.unwrap_or(0),
                        best_before: None,
                        barcode: None,
                        collected: false,
                        product: Some(product),
                    };
                    insert_purchase_item(tx.as_mut(), new_purchase_id, &item).await?;
                }

                purchase_id = Some(new_purchase_id);
            }
        }

        let r = sqlx::query(
            r#"
            UPDATE inventory_check
            SET state = 'completed', completed_at = now(), completed_by_account_id = $2,
                generated_purchase_id = $3
            WHERE id = $1
            "#,
        )
        .bind(to_i64(id))
        .bind(completion.completed_by_account_id.map(to_i64))
        .bind(purchase_id.map(to_i64))
        .execute(tx.as_mut())
        .await;
        to_service_result(r)?;

        to_service_result(tx.commit().await)?;

        let check = fetch_check(self.connection.as_mut(), id)
            .await?
            .ok_or(ServiceError::NotFound)?;
        let purchase = match purchase_id {
            Some(purchase_id) => fetch_purchase(self.connection.as_mut(), purchase_id).await?,
            None => None,
        };

        Ok((check, purchase))
    }
}
