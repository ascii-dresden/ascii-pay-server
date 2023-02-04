use axum::Router;

use crate::database::Database;

mod accounts;

pub fn init() -> Router<Database> {
    Router::new().merge(accounts::router())
}
