use aide::axum::ApiRouter;

use crate::database::AppState;

mod accounts;
mod auth;
mod products;
mod transactions;

pub fn init(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .merge(accounts::router(app_state.clone()))
        .merge(auth::router(app_state.clone()))
        .merge(products::router(app_state.clone()))
        .merge(transactions::router(app_state))
}
