use crate::{database::DatabaseConnection, error::ServiceResult};

use self::{
    product_import::load_product_repo_into_database, sql_dump_import::load_sql_dump_into_database,
};

mod product_import;
mod sql_dump_import;

pub async fn import(
    db: &mut DatabaseConnection,
    products_path: &str,
    sql_dump_path: &str,
) -> ServiceResult<()> {
    let product_map = load_product_repo_into_database(db, products_path).await?;
    load_sql_dump_into_database(db, sql_dump_path, product_map).await?;

    Ok(())
}
