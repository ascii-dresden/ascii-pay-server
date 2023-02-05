use aide::axum::ApiRouter;

use crate::database::AppState;

mod account_auth_methods;
mod accounts;
mod auth;
mod mifare;
mod products;
mod transactions;

pub fn init(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .merge(account_auth_methods::router(app_state.clone()))
        .merge(accounts::router(app_state.clone()))
        .merge(auth::router(app_state.clone()))
        .merge(products::router(app_state.clone()))
        .merge(transactions::router(app_state))
}
