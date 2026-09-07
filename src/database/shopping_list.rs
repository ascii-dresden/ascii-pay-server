//! Shopping list persistence: shared wish list entries that purchasers tick off.

use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::{Connection, FromRow, PgConnection, Row};

use crate::error::{ServiceError, ServiceResult};
use crate::models;

use super::{
    to_service_result, DatabaseConnection, ProductRow, PRODUCT_COLUMNS, PRODUCT_STATUS_JOINS,
};

const ITEM_DONE: &str = "Shopping list item is done";
const ITEM_NOT_DONE: &str = "Shopping list item is not done";

#[derive(sqlx::FromRow)]
struct ShoppingListItemRow {
    #[sqlx(try_from = "i64")]
    sli_id: u64,
    sli_name: String,
    sli_quantity: i32,
    sli_note: String,
    sli_created_by_account_id: Option<i64>,
    sli_created_by_name: Option<String>,
    sli_created_at: DateTime<Utc>,
    sli_done_at: Option<DateTime<Utc>>,
    sli_done_by_account_id: Option<i64>,
    sli_purchase_id: Option<i64>,
}

fn to_u64(id: i64) -> u64 {
    id.try_into().expect("IDs are non-negative")
}

fn to_i64(id: u64) -> i64 {
    i64::try_from(id).expect("id is always less than 2**63")
}

/// Builds the select for shopping list items including the (optional) product and the
/// creator's name. The item table is aliased as `sli`.
fn item_query(where_clause: &str, order_clause: &str) -> String {
    format!(
        r#"
        SELECT
            sli.id as sli_id,
            sli.name as sli_name,
            sli.quantity as sli_quantity,
            sli.note as sli_note,
            sli.created_by_account_id as sli_created_by_account_id,
            creator.name as sli_created_by_name,
            sli.created_at as sli_created_at,
            sli.done_at as sli_done_at,
            sli.done_by_account_id as sli_done_by_account_id,
            sli.purchase_id as sli_purchase_id,
            {PRODUCT_COLUMNS}
        FROM shopping_list_item sli
            LEFT OUTER JOIN account creator ON sli.created_by_account_id = creator.id
            LEFT OUTER JOIN product p ON sli.product_id = p.id
            {PRODUCT_STATUS_JOINS}
        WHERE {where_clause}
        GROUP BY sli.id, creator.id, p.id
        ORDER BY {order_clause}
        "#
    )
}

fn item_from_row(row: &PgRow) -> ServiceResult<models::ShoppingListItem> {
    let item_row = to_service_result(ShoppingListItemRow::from_row(row))?;
    let mut item = models::ShoppingListItem {
        id: item_row.sli_id,
        name: item_row.sli_name,
        quantity: item_row.sli_quantity,
        note: item_row.sli_note,
        product: None,
        created_by_account_id: item_row.sli_created_by_account_id.map(to_u64),
        created_by_name: item_row.sli_created_by_name,
        created_at: item_row.sli_created_at,
        done_at: item_row.sli_done_at,
        done_by_account_id: item_row.sli_done_by_account_id.map(to_u64),
        purchase_id: item_row.sli_purchase_id.map(to_u64),
    };

    let product_id: Option<i64> = row.get("id");
    if product_id.is_some() {
        item.product = Some(to_service_result(ProductRow::from_row(row))?.into());
    }

    Ok(item)
}

async fn fetch_item(
    conn: &mut PgConnection,
    id: u64,
) -> ServiceResult<Option<models::ShoppingListItem>> {
    let r = sqlx::query(&item_query("sli.id = $1", "sli.id ASC"))
        .bind(to_i64(id))
        .fetch_optional(conn)
        .await;

    match to_service_result(r)? {
        Some(row) => Ok(Some(item_from_row(&row)?)),
        None => Ok(None),
    }
}

/// Locks the item row for the rest of the transaction and returns whether it is done.
async fn lock_item(conn: &mut PgConnection, id: u64) -> ServiceResult<bool> {
    let r = sqlx::query(
        "SELECT done_at IS NOT NULL AS done FROM shopping_list_item WHERE id = $1 FOR UPDATE",
    )
    .bind(to_i64(id))
    .fetch_optional(conn)
    .await;

    match to_service_result(r)? {
        Some(row) => Ok(row.get("done")),
        None => Err(ServiceError::NotFound),
    }
}

async fn purchase_exists(conn: &mut PgConnection, purchase_id: u64) -> ServiceResult<bool> {
    let r = sqlx::query("SELECT 1 FROM purchase WHERE id = $1")
        .bind(to_i64(purchase_id))
        .fetch_optional(conn)
        .await;
    Ok(to_service_result(r)?.is_some())
}

impl DatabaseConnection {
    /// Lists shopping list items. Open items are ordered oldest first, done items newest
    /// (most recently done) first; with `All` the open items come before the done ones.
    pub async fn get_shopping_list_items(
        &mut self,
        state: models::ShoppingListState,
    ) -> ServiceResult<Vec<models::ShoppingListItem>> {
        let (where_clause, order_clause) = match state {
            models::ShoppingListState::Open => {
                ("sli.done_at IS NULL", "sli.created_at ASC, sli.id ASC")
            }
            models::ShoppingListState::Done => {
                ("sli.done_at IS NOT NULL", "sli.done_at DESC, sli.id DESC")
            }
            models::ShoppingListState::All => (
                "TRUE",
                "sli.done_at IS NOT NULL ASC, sli.done_at DESC, sli.created_at ASC, sli.id ASC",
            ),
        };

        let r = sqlx::query(&item_query(where_clause, order_clause))
            .fetch_all(self.connection.as_mut())
            .await;

        to_service_result(r)?.iter().map(item_from_row).collect()
    }

    pub async fn get_shopping_list_item_by_id(
        &mut self,
        id: u64,
    ) -> ServiceResult<Option<models::ShoppingListItem>> {
        fetch_item(self.connection.as_mut(), id).await
    }

    /// Creates a new open item. `id`, `created_by_name`, `created_at` and the done fields of
    /// `item` are ignored.
    pub async fn create_shopping_list_item(
        &mut self,
        item: models::ShoppingListItem,
    ) -> ServiceResult<models::ShoppingListItem> {
        let r = sqlx::query(
            r#"
            INSERT INTO shopping_list_item (name, quantity, note, product_id, created_by_account_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(item.name.as_str())
        .bind(item.quantity)
        .bind(item.note.as_str())
        .bind(item.product.as_ref().map(|p| to_i64(p.id)))
        .bind(item.created_by_account_id.map(to_i64))
        .fetch_one(self.connection.as_mut())
        .await;
        let id = to_u64(to_service_result(r)?.get::<i64, _>(0));

        fetch_item(self.connection.as_mut(), id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Updates `name`, `quantity`, `note` and `product` of an open item.
    ///
    /// Fails with `Conflict` if the item is done.
    pub async fn update_shopping_list_item(
        &mut self,
        item: models::ShoppingListItem,
    ) -> ServiceResult<models::ShoppingListItem> {
        let mut tx = self.connection.begin().await?;

        if lock_item(tx.as_mut(), item.id).await? {
            return Err(ServiceError::Conflict(ITEM_DONE));
        }

        let r = sqlx::query(
            r#"
            UPDATE shopping_list_item
            SET name = $2, quantity = $3, note = $4, product_id = $5
            WHERE id = $1
            "#,
        )
        .bind(to_i64(item.id))
        .bind(item.name.as_str())
        .bind(item.quantity)
        .bind(item.note.as_str())
        .bind(item.product.as_ref().map(|p| to_i64(p.id)))
        .execute(tx.as_mut())
        .await;
        to_service_result(r)?;

        to_service_result(tx.commit().await)?;

        fetch_item(self.connection.as_mut(), item.id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    pub async fn delete_shopping_list_item(&mut self, id: u64) -> ServiceResult<()> {
        let r = sqlx::query("DELETE FROM shopping_list_item WHERE id = $1")
            .bind(to_i64(id))
            .execute(self.connection.as_mut())
            .await;
        if to_service_result(r)?.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    /// Marks an open item as done, optionally linking the purchase it was bought with.
    ///
    /// Fails with `Conflict` if the item is already done and with `NotFound` if the item or
    /// the referenced purchase does not exist.
    pub async fn mark_shopping_list_item_done(
        &mut self,
        id: u64,
        done_by_account_id: Option<u64>,
        purchase_id: Option<u64>,
    ) -> ServiceResult<models::ShoppingListItem> {
        let mut tx = self.connection.begin().await?;

        if lock_item(tx.as_mut(), id).await? {
            return Err(ServiceError::Conflict(ITEM_DONE));
        }

        if let Some(purchase_id) = purchase_id {
            if !purchase_exists(tx.as_mut(), purchase_id).await? {
                return Err(ServiceError::NotFound);
            }
        }

        let r = sqlx::query(
            r#"
            UPDATE shopping_list_item
            SET done_at = now(), done_by_account_id = $2, purchase_id = $3
            WHERE id = $1
            "#,
        )
        .bind(to_i64(id))
        .bind(done_by_account_id.map(to_i64))
        .bind(purchase_id.map(to_i64))
        .execute(tx.as_mut())
        .await;
        to_service_result(r)?;

        to_service_result(tx.commit().await)?;

        fetch_item(self.connection.as_mut(), id)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// Reopens a done item and clears the done fields.
    ///
    /// Fails with `Conflict` if the item is open.
    pub async fn reopen_shopping_list_item(
        &mut self,
        id: u64,
    ) -> ServiceResult<models::ShoppingListItem> {
        let mut tx = self.connection.begin().await?;

        if !lock_item(tx.as_mut(), id).await? {
            return Err(ServiceError::Conflict(ITEM_NOT_DONE));
        }

        let r = sqlx::query(
            r#"
            UPDATE shopping_list_item
            SET done_at = NULL, done_by_account_id = NULL, purchase_id = NULL
            WHERE id = $1
            "#,
        )
        .bind(to_i64(id))
        .execute(tx.as_mut())
        .await;
        to_service_result(r)?;

        to_service_result(tx.commit().await)?;

        fetch_item(self.connection.as_mut(), id)
            .await?
            .ok_or(ServiceError::NotFound)
    }
}
