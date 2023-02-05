use aide::axum::ApiRouter;
use argon2rs::verifier::Encoded;

use crate::{database::AppState, error::ServiceResult};

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

fn password_hash_create(password: &str) -> ServiceResult<Vec<u8>> {
    let bytes =
        Encoded::default2i(password.as_bytes(), "SALTSALTSALT".as_bytes(), b"", b"").to_u8();
    Ok(bytes)
}

fn password_hash_verify(hash: &[u8], password: &str) -> ServiceResult<bool> {
    if let Ok(enc) = Encoded::from_u8(hash) {
        return Ok(enc.verify(password.as_bytes()));
    }

    Ok(false)
}
