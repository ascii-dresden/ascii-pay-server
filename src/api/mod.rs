use aide::axum::ApiRouter;
use argon2rs::verifier::Encoded;
use rand::RngCore;

use crate::{database::AppState, error::ServiceResult};

mod account_auth_methods;
mod account_status;
mod accounts;
mod auth;
mod nfc_id;
mod nfc_mifare;
mod products;
mod register;
mod report;
mod transactions;

pub mod wallet_routes;

pub fn init(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .merge(account_auth_methods::router(app_state.clone()))
        .merge(account_status::router(app_state.clone()))
        .merge(accounts::router(app_state.clone()))
        .merge(auth::router(app_state.clone()))
        .merge(products::router(app_state.clone()))
        .merge(register::router(app_state.clone()))
        .merge(transactions::router(app_state.clone()))
        .merge(report::router(app_state))
}

fn password_hash_create(password: &str) -> ServiceResult<Vec<u8>> {
    let mut data = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut data);
    let bytes = Encoded::default2i(password.as_bytes(), &data, b"", b"").to_u8();
    Ok(bytes)
}

fn password_hash_verify(hash: &[u8], password: &str) -> ServiceResult<bool> {
    if let Ok(enc) = Encoded::from_u8(hash) {
        return Ok(enc.verify(password.as_bytes()));
    }

    Ok(false)
}
