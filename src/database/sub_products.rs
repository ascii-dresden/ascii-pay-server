//! Sub product persistence: purchasing variants that are tracked on their own but sold as the
//! parent product.

use sqlx::{Connection, PgConnection};

use crate::error::{ServiceError, ServiceResult};
use crate::models;

use super::{to_service_result, DatabaseConnection};

#[derive(sqlx::FromRow)]
struct SubProductRow {
    id: i64,
    name: String,
    purchase_tax: i32,
    active: bool,
    show_on_print_list: bool,
    target_quantity: i32,
}

impl From<SubProductRow> for models::SubProduct {
    fn from(value: SubProductRow) -> Self {
        models::SubProduct {
            id: to_u64(value.id),
            name: value.name,
            purchase_tax: value.purchase_tax,
            active: value.active,
            show_on_print_list: value.show_on_print_list,
            target_quantity: value.target_quantity,
        }
    }
}

fn to_u64(id: i64) -> u64 {
    id.try_into().expect("IDs are non-negative")
}

fn to_i64(id: u64) -> i64 {
    i64::try_from(id).expect("id is always less than 2**63")
}

fn clean_name(name: &str) -> ServiceResult<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ServiceError::BadRequest(
            "sub product name must not be empty".to_owned(),
        ));
    }
    Ok(name)
}

async fn product_exists(conn: &mut PgConnection, product_id: u64) -> ServiceResult<bool> {
    let r = sqlx::query("SELECT 1 FROM product WHERE id = $1")
        .bind(to_i64(product_id))
        .fetch_optional(conn)
        .await;
    Ok(to_service_result(r)?.is_some())
}

async fn insert_sub_product(
    conn: &mut PgConnection,
    product_id: u64,
    sub_product: &models::SubProduct,
) -> ServiceResult<models::SubProduct> {
    let r = sqlx::query_as::<_, SubProductRow>(
        r#"
        INSERT INTO sub_product (
            product_id, name, purchase_tax, active, show_on_print_list, target_quantity
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, name, purchase_tax, active, show_on_print_list, target_quantity
        "#,
    )
    .bind(to_i64(product_id))
    .bind(clean_name(&sub_product.name)?)
    .bind(sub_product.purchase_tax)
    .bind(sub_product.active)
    .bind(sub_product.show_on_print_list)
    .bind(sub_product.target_quantity.max(0))
    .fetch_one(conn)
    .await;
    Ok(to_service_result(r)?.into())
}

/// Updates a sub product; `None` if it does not belong to the product.
async fn update_sub_product_row(
    conn: &mut PgConnection,
    product_id: u64,
    sub_product: &models::SubProduct,
) -> ServiceResult<Option<models::SubProduct>> {
    let r = sqlx::query_as::<_, SubProductRow>(
        r#"
        UPDATE sub_product
        SET name = $3, purchase_tax = $4, active = $5, show_on_print_list = $6,
            target_quantity = $7
        WHERE id = $1 AND product_id = $2
        RETURNING id, name, purchase_tax, active, show_on_print_list, target_quantity
        "#,
    )
    .bind(to_i64(sub_product.id))
    .bind(to_i64(product_id))
    .bind(clean_name(&sub_product.name)?)
    .bind(sub_product.purchase_tax)
    .bind(sub_product.active)
    .bind(sub_product.show_on_print_list)
    .bind(sub_product.target_quantity.max(0))
    .fetch_optional(conn)
    .await;
    Ok(to_service_result(r)?.map(Into::into))
}

impl DatabaseConnection {
    /// Replaces the sub products of a product with the given list and returns the stored rows
    /// in input order. Entries with `id == 0` are created, the others updated.
    ///
    /// Sub products missing from the list are deleted when nothing was ever bought as them;
    /// otherwise they are only deactivated, so their purchase history keeps its name.
    ///
    /// Fails with `BadRequest` (and stores nothing) if an id belongs to another product or a
    /// name is empty.
    pub async fn store_sub_products(
        &mut self,
        product_id: u64,
        sub_products: &[models::SubProduct],
    ) -> ServiceResult<Vec<models::SubProduct>> {
        let mut tx = self.connection.begin().await?;

        let mut stored = Vec::with_capacity(sub_products.len());
        for sub_product in sub_products {
            let row = if sub_product.id == 0 {
                insert_sub_product(tx.as_mut(), product_id, sub_product).await?
            } else {
                update_sub_product_row(tx.as_mut(), product_id, sub_product)
                    .await?
                    .ok_or_else(|| {
                        ServiceError::BadRequest(
                            "sub product does not belong to the product".to_owned(),
                        )
                    })?
            };
            stored.push(row);
        }

        let keep: Vec<i64> = stored.iter().map(|s| to_i64(s.id)).collect();
        let r = sqlx::query(
            r#"
            DELETE FROM sub_product sp
            WHERE sp.product_id = $1
                AND NOT (sp.id = ANY($2::BIGINT[]))
                AND NOT EXISTS (SELECT 1 FROM purchase_item pi WHERE pi.sub_product_id = sp.id)
            "#,
        )
        .bind(to_i64(product_id))
        .bind(&keep)
        .execute(tx.as_mut())
        .await;
        to_service_result(r)?;

        let r = sqlx::query(
            r#"
            UPDATE sub_product SET active = FALSE
            WHERE product_id = $1 AND NOT (id = ANY($2::BIGINT[]))
            "#,
        )
        .bind(to_i64(product_id))
        .bind(&keep)
        .execute(tx.as_mut())
        .await;
        to_service_result(r)?;

        to_service_result(tx.commit().await)?;
        Ok(stored)
    }

    /// Adds a single sub product without rewriting the product. `NotFound` if the product
    /// does not exist.
    pub async fn add_sub_product(
        &mut self,
        product_id: u64,
        sub_product: &models::SubProduct,
    ) -> ServiceResult<models::SubProduct> {
        if !product_exists(self.connection.as_mut(), product_id).await? {
            return Err(ServiceError::NotFound);
        }
        insert_sub_product(self.connection.as_mut(), product_id, sub_product).await
    }

    /// Updates a single sub product. `NotFound` if it does not belong to the product.
    pub async fn update_sub_product(
        &mut self,
        product_id: u64,
        sub_product: &models::SubProduct,
    ) -> ServiceResult<models::SubProduct> {
        update_sub_product_row(self.connection.as_mut(), product_id, sub_product)
            .await?
            .ok_or(ServiceError::NotFound)
    }
}
