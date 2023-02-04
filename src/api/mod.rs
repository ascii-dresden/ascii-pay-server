use aide::axum::ApiRouter;

use crate::database::Database;

mod accounts;
mod products;
mod transactions;

pub fn init(database: Database) -> ApiRouter {
    ApiRouter::new()
        .merge(accounts::router(database.clone()))
        .merge(products::router(database.clone()))
        .merge(transactions::router(database))
}
