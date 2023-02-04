use axum::Router;

use crate::database::Database;

mod accounts;
mod products;

pub fn init() -> Router<Database> {
    Router::new()
        .merge(accounts::router())
        .merge(products::router())
}
