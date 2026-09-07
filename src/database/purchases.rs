//! Purchase persistence: purchases, purchase items and the draft/finalized lifecycle.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::postgres::PgRow;
use sqlx::{Connection, FromRow, PgConnection, Row};

use crate::error::{ServiceError, ServiceResult};
use crate::models;

use super::{
    to_service_result, DatabaseConnection, ProductRow, PurchaseStateDto, PRODUCT_COLUMNS,
    PRODUCT_STATUS_JOINS,
};

const PURCHASE_FINALIZED: &str = "Purchase is finalized";
const PURCHASE_NOT_FINALIZED: &str = "Purchase is not finalized";

const PURCHASE_COLUMNS: &str = r#"
    purchase.id, purchase.name, purchase.purchased_by_account_id, purchase.store, purchase.timestamp,
    purchase.state, purchase.finalized_at, purchase.finalized_by_account_id
"#;

#[derive(sqlx::FromRow)]
struct PurchaseRow {
    #[sqlx(try_from = "i64")]
    id: u64,
    name: String,
    purchased_by_account_id: Option<i64>,
    store: String,
    timestamp: DateTime<Utc>,
    state: PurchaseStateDto,
    finalized_at: Option<DateTime<Utc>>,
    finalized_by_account_id: Option<i64>,
}

impl From<PurchaseRow> for models::Purchase {
    fn from(value: PurchaseRow) -> Self {
        models::Purchase {
            id: value.id,
            name: value.name,
            timestamp: value.timestamp,
            store: value.store,
            state: value.state.into(),
            purchased_by_account_id: value.purchased_by_account_id.map(to_u64),
            finalized_at: value.finalized_at,
            finalized_by_account_id: value.finalized_by_account_id.map(to_u64),
            items: Vec::new(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct PurchaseItemRow {
    #[sqlx(try_from = "i64")]
    purchase_item_id: u64,
    #[sqlx(try_from = "i64")]
    purchase_item_purchase_id: u64,
    purchase_item_name: String,
    purchase_item_container_size: i32,
    purchase_item_container_count: i32,
    purchase_item_container_cents: i32,
    purchase_item_best_before: Option<NaiveDate>,
    purchase_item_barcode: Option<String>,
    purchase_item_collected: bool,
}

fn to_u64(id: i64) -> u64 {
    id.try_into().expect("IDs are non-negative")
}

fn to_i64(id: u64) -> i64 {
    i64::try_from(id).expect("id is always less than 2**63")
}

fn purchase_item_query(where_clause: &str) -> String {
    format!(
        r#"
        SELECT
            item.id as purchase_item_id,
            item.purchase_id as purchase_item_purchase_id,
            item.name as purchase_item_name,
            item.container_size as purchase_item_container_size,
            item.container_count as purchase_item_container_count,
            item.container_cents as purchase_item_container_cents,
            item.best_before as purchase_item_best_before,
            item.barcode as purchase_item_barcode,
            item.collected as purchase_item_collected,
            {PRODUCT_COLUMNS}
        FROM purchase_item item
            LEFT OUTER JOIN product p ON item.product_id = p.id
            {PRODUCT_STATUS_JOINS}
        WHERE {where_clause}
        GROUP BY item.id, p.id
        ORDER BY item.created_at ASC, item.id ASC
        "#
    )
}

/// Maps a row of `purchase_item_query` into `(purchase_id, item)`.
fn purchase_item_from_row(row: &PgRow) -> ServiceResult<(u64, models::PurchaseItem)> {
    let item_row = to_service_result(PurchaseItemRow::from_row(row))?;
    let mut item = models::PurchaseItem {
        id: item_row.purchase_item_id,
        name: item_row.purchase_item_name,
        container_size: item_row.purchase_item_container_size,
        container_count: item_row.purchase_item_container_count,
        container_cents: item_row.purchase_item_container_cents,
        best_before: item_row.purchase_item_best_before,
        barcode: item_row.purchase_item_barcode,
        collected: item_row.purchase_item_collected,
        product: None,
    };

    let product_id: Option<i64> = row.get("id");
    if product_id.is_some() {
        item.product = Some(to_service_result(ProductRow::from_row(row))?.into());
    }

    Ok((item_row.purchase_item_purchase_id, item))
}

async fn load_purchase_items(
    conn: &mut PgConnection,
    purchases: &mut [models::Purchase],
) -> ServiceResult<()> {
    if purchases.is_empty() {
        return Ok(());
    }

    let purchase_ids: Vec<i64> = purchases.iter().map(|p| to_i64(p.id)).collect();
    let rows = sqlx::query(&purchase_item_query("item.purchase_id = ANY($1::BIGINT[])"))
        .bind(purchase_ids)
        .fetch_all(conn)
        .await;

    for row in to_service_result(rows)? {
        let (purchase_id, item) = purchase_item_from_row(&row)?;
        if let Some(purchase) = purchases.iter_mut().find(|p| p.id == purchase_id) {
            purchase.items.push(item);
        }
    }

    Ok(())
}

pub(super) async fn fetch_purchase(
    conn: &mut PgConnection,
    id: u64,
) -> ServiceResult<Option<models::Purchase>> {
    let r = sqlx::query_as::<_, PurchaseRow>(&format!(
        "SELECT {PURCHASE_COLUMNS} FROM purchase WHERE purchase.id = $1"
    ))
    .bind(to_i64(id))
    .fetch_optional(&mut *conn)
    .await;

    let Some(row) = to_service_result(r)? else {
        return Ok(None);
    };

    let mut purchases = vec![models::Purchase::from(row)];
    load_purchase_items(conn, &mut purchases).await?;
    Ok(purchases.pop())
}

async fn fetch_purchase_item(
    conn: &mut PgConnection,
    purchase_id: u64,
    item_id: u64,
) -> ServiceResult<Option<models::PurchaseItem>> {
    let r = sqlx::query(&purchase_item_query(
        "item.purchase_id = $1 AND item.id = $2",
    ))
    .bind(to_i64(purchase_id))
    .bind(to_i64(item_id))
    .fetch_optional(conn)
    .await;

    match to_service_result(r)? {
        Some(row) => Ok(Some(purchase_item_from_row(&row)?.1)),
        None => Ok(None),
    }
}

/// Locks the purchase row for the rest of the transaction and returns its state.
async fn lock_purchase(conn: &mut PgConnection, id: u64) -> ServiceResult<PurchaseStateDto> {
    let r = sqlx::query("SELECT state FROM purchase WHERE id = $1 FOR UPDATE")
        .bind(to_i64(id))
        .fetch_optional(conn)
        .await;

    match to_service_result(r)? {
        Some(row) => Ok(row.get("state")),
        None => Err(ServiceError::NotFound),
    }
}

/// Locks the purchase and fails with `Conflict` unless it is a draft.
async fn lock_draft_purchase(conn: &mut PgConnection, id: u64) -> ServiceResult<()> {
    match lock_purchase(conn, id).await? {
        PurchaseStateDto::Draft => Ok(()),
        PurchaseStateDto::Finalized => Err(ServiceError::Conflict(PURCHASE_FINALIZED)),
    }
}

/// Inserts the purchase header (without items) and returns the new id.
pub(super) async fn insert_purchase(
    conn: &mut PgConnection,
    purchase: &models::Purchase,
) -> ServiceResult<u64> {
    let finalized_at = match purchase.state {
        models::PurchaseState::Draft => None,
        models::PurchaseState::Finalized => Some(purchase.finalized_at.unwrap_or_else(Utc::now)),
    };
    let finalized_by = match purchase.state {
        models::PurchaseState::Draft => None,
        models::PurchaseState::Finalized => purchase.finalized_by_account_id.map(to_i64),
    };

    let r = sqlx::query(
        r#"
        INSERT INTO purchase (
            name, purchased_by_account_id, store, timestamp, state, finalized_at, finalized_by_account_id
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
    )
    .bind(purchase.name.as_str())
    .bind(purchase.purchased_by_account_id.map(to_i64))
    .bind(purchase.store.as_str())
    .bind(purchase.timestamp)
    .bind(PurchaseStateDto::from(purchase.state))
    .bind(finalized_at)
    .bind(finalized_by)
    .fetch_one(conn)
    .await;

    Ok(to_u64(to_service_result(r)?.get::<i64, _>(0)))
}

/// Inserts one item into the given purchase and returns the new item id.
pub(super) async fn insert_purchase_item(
    conn: &mut PgConnection,
    purchase_id: u64,
    item: &models::PurchaseItem,
) -> ServiceResult<u64> {
    let r = sqlx::query(
        r#"
        INSERT INTO purchase_item (
            purchase_id, name, container_size, container_count, container_cents,
            product_id, best_before, barcode, collected
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id
        "#,
    )
    .bind(to_i64(purchase_id))
    .bind(item.name.as_str())
    .bind(item.container_size)
    .bind(item.container_count)
    .bind(item.container_cents)
    .bind(item.product.as_ref().map(|p| to_i64(p.id)))
    .bind(item.best_before)
    .bind(item.barcode.as_deref())
    .bind(item.collected)
    .fetch_one(conn)
    .await;

    Ok(to_u64(to_service_result(r)?.get::<i64, _>(0)))
}

async fn insert_purchase_items(
    conn: &mut PgConnection,
    purchase_id: u64,
    items: &[models::PurchaseItem],
) -> ServiceResult<()> {
    for item in items {
        insert_purchase_item(&mut *conn, purchase_id, item).await?;
    }
    Ok(())
}

impl DatabaseConnection {
    /// Lists purchases, newest first, optionally filtered by state.
    pub async fn get_purchases(
        &mut self,
        state: Option<models::PurchaseState>,
    ) -> ServiceResult<Vec<models::Purchase>> {
        let r = sqlx::query_as::<_, PurchaseRow>(&format!(
            r#"
            SELECT {PURCHASE_COLUMNS}
            FROM purchase
            WHERE $1::tp_purchase_state IS NULL OR purchase.state = $1
            ORDER BY purchase.timestamp DESC, purchase.id DESC
            "#
        ))
        .bind(state.map(PurchaseStateDto::from))
        .fetch_all(self.connection.as_mut())
        .await;

        let mut out: Vec<models::Purchase> =
            to_service_result(r)?.into_iter().map(Into::into).collect();
        load_purchase_items(self.connection.as_mut(), &mut out).await?;
        Ok(out)
    }

    pub async fn get_purchase_by_id(&mut self, id: u64) -> ServiceResult<Option<models::Purchase>> {
        fetch_purchase(self.connection.as_mut(), id).await
    }

    /// Returns `None` if the product does not exist.
    pub async fn get_purchases_by_product_id(
        &mut self,
        product_id: u64,
    ) -> ServiceResult<Option<Vec<models::Purchase>>> {
        if self.get_product_by_id(product_id).await?.is_none() {
            return Ok(None);
        }

        let r = sqlx::query_as::<_, PurchaseRow>(&format!(
            r#"
            SELECT {PURCHASE_COLUMNS}
            FROM purchase
            WHERE EXISTS (
                SELECT 1 FROM purchase_item
                WHERE purchase_item.purchase_id = purchase.id AND purchase_item.product_id = $1
            )
            ORDER BY purchase.timestamp DESC, purchase.id DESC
            "#
        ))
        .bind(to_i64(product_id))
        .fetch_all(self.connection.as_mut())
        .await;

        let mut out: Vec<models::Purchase> =
            to_service_result(r)?.into_iter().map(Into::into).collect();
        load_purchase_items(self.connection.as_mut(), &mut out).await?;
        Ok(Some(out))
    }

    /// Creates a new purchase including its items. `purchase.id` is ignored.
    pub async fn create_purchase(
        &mut self,
        purchase: models::Purchase,
    ) -> ServiceResult<models::Purchase> {
        let mut tx = self.connection.begin().await?;

        let purchase_id = insert_purchase(tx.as_mut(), &purchase).await?;
        insert_purchase_items(tx.as_mut(), purchase_id, &purchase.items).await?;

        to_service_result(tx.commit().await)?;

        fetch_purchase(self.connection.as_mut(), purchase_id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Updates the header fields of a draft purchase. When `replace_items` is set the item list
    /// is replaced by `purchase.items`, otherwise the items are left untouched.
    ///
    /// Fails with `Conflict` if the purchase is finalized.
    pub async fn update_purchase(
        &mut self,
        purchase: models::Purchase,
        replace_items: bool,
    ) -> ServiceResult<models::Purchase> {
        let mut tx = self.connection.begin().await?;

        lock_draft_purchase(tx.as_mut(), purchase.id).await?;

        let r = sqlx::query(
            r#"
            UPDATE purchase
            SET name = $2, purchased_by_account_id = $3, store = $4, timestamp = $5
            WHERE id = $1
            "#,
        )
        .bind(to_i64(purchase.id))
        .bind(purchase.name.as_str())
        .bind(purchase.purchased_by_account_id.map(to_i64))
        .bind(purchase.store.as_str())
        .bind(purchase.timestamp)
        .execute(tx.as_mut())
        .await;
        to_service_result(r)?;

        if replace_items {
            let r = sqlx::query("DELETE FROM purchase_item WHERE purchase_id = $1")
                .bind(to_i64(purchase.id))
                .execute(tx.as_mut())
                .await;
            to_service_result(r)?;
            insert_purchase_items(tx.as_mut(), purchase.id, &purchase.items).await?;
        }

        to_service_result(tx.commit().await)?;

        fetch_purchase(self.connection.as_mut(), purchase.id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    pub async fn delete_purchase(&mut self, id: u64) -> ServiceResult<()> {
        let r = sqlx::query(r#"DELETE FROM purchase WHERE id = $1"#)
            .bind(to_i64(id))
            .execute(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;
        if r.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    /// Finalizes a draft purchase: drops uncollected items and records who finalized it.
    ///
    /// Fails with `Conflict` if the purchase is already finalized.
    pub async fn finalize_purchase(
        &mut self,
        id: u64,
        finalized_by_account_id: Option<u64>,
    ) -> ServiceResult<models::Purchase> {
        let mut tx = self.connection.begin().await?;

        lock_draft_purchase(tx.as_mut(), id).await?;

        let r = sqlx::query("DELETE FROM purchase_item WHERE purchase_id = $1 AND NOT collected")
            .bind(to_i64(id))
            .execute(tx.as_mut())
            .await;
        to_service_result(r)?;

        let r = sqlx::query(
            r#"
            UPDATE purchase
            SET state = 'finalized', finalized_at = now(), finalized_by_account_id = $2
            WHERE id = $1
            "#,
        )
        .bind(to_i64(id))
        .bind(finalized_by_account_id.map(to_i64))
        .execute(tx.as_mut())
        .await;
        to_service_result(r)?;

        to_service_result(tx.commit().await)?;

        fetch_purchase(self.connection.as_mut(), id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Reopens a finalized purchase as a draft.
    ///
    /// Fails with `Conflict` if the purchase is a draft.
    pub async fn reopen_purchase(&mut self, id: u64) -> ServiceResult<models::Purchase> {
        let mut tx = self.connection.begin().await?;

        match lock_purchase(tx.as_mut(), id).await? {
            PurchaseStateDto::Finalized => {}
            PurchaseStateDto::Draft => return Err(ServiceError::Conflict(PURCHASE_NOT_FINALIZED)),
        }

        let r = sqlx::query(
            r#"
            UPDATE purchase
            SET state = 'draft', finalized_at = NULL, finalized_by_account_id = NULL
            WHERE id = $1
            "#,
        )
        .bind(to_i64(id))
        .execute(tx.as_mut())
        .await;
        to_service_result(r)?;

        to_service_result(tx.commit().await)?;

        fetch_purchase(self.connection.as_mut(), id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Adds an item to a draft purchase. `item.id` is ignored.
    pub async fn add_purchase_item(
        &mut self,
        purchase_id: u64,
        item: models::PurchaseItem,
    ) -> ServiceResult<models::PurchaseItem> {
        let mut tx = self.connection.begin().await?;

        lock_draft_purchase(tx.as_mut(), purchase_id).await?;
        let item_id = insert_purchase_item(tx.as_mut(), purchase_id, &item).await?;

        to_service_result(tx.commit().await)?;

        fetch_purchase_item(self.connection.as_mut(), purchase_id, item_id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Updates an item of a draft purchase. Fails with `NotFound` if the item does not belong
    /// to the purchase.
    pub async fn update_purchase_item(
        &mut self,
        purchase_id: u64,
        item_id: u64,
        item: models::PurchaseItem,
    ) -> ServiceResult<models::PurchaseItem> {
        let mut tx = self.connection.begin().await?;

        lock_draft_purchase(tx.as_mut(), purchase_id).await?;

        let r = sqlx::query(
            r#"
            UPDATE purchase_item
            SET name = $3, container_size = $4, container_count = $5, container_cents = $6,
                product_id = $7, best_before = $8, barcode = $9, collected = $10
            WHERE purchase_id = $1 AND id = $2
            "#,
        )
        .bind(to_i64(purchase_id))
        .bind(to_i64(item_id))
        .bind(item.name.as_str())
        .bind(item.container_size)
        .bind(item.container_count)
        .bind(item.container_cents)
        .bind(item.product.as_ref().map(|p| to_i64(p.id)))
        .bind(item.best_before)
        .bind(item.barcode.as_deref())
        .bind(item.collected)
        .execute(tx.as_mut())
        .await;
        if to_service_result(r)?.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }

        to_service_result(tx.commit().await)?;

        fetch_purchase_item(self.connection.as_mut(), purchase_id, item_id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Deletes an item of a draft purchase.
    pub async fn delete_purchase_item(
        &mut self,
        purchase_id: u64,
        item_id: u64,
    ) -> ServiceResult<()> {
        let mut tx = self.connection.begin().await?;

        lock_draft_purchase(tx.as_mut(), purchase_id).await?;

        let r = sqlx::query("DELETE FROM purchase_item WHERE purchase_id = $1 AND id = $2")
            .bind(to_i64(purchase_id))
            .bind(to_i64(item_id))
            .execute(tx.as_mut())
            .await;
        if to_service_result(r)?.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }

        to_service_result(tx.commit().await)?;
        Ok(())
    }
}
