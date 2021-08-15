use actix_web::{web, HttpResponse};

use crate::{
    identity_service::Identity,
    model::{wallet, Pool, ServiceResult},
    repo::{self, PaymentInput, TokenInput},
};

/// POST route for `/api/v1/transaction/token`
pub async fn post_transaction_token(
    pool: web::Data<Pool>,
    identity: Identity,
    input: web::Json<TokenInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::transaction_token(conn, &identity, input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// POST route for `/api/v1/transaction/payment`
pub async fn post_transaction_payment(
    pool: web::Data<Pool>,
    identity: Identity,
    input: web::Json<PaymentInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::transaction_payment(conn, &identity, input.into_inner())?;

    if let Err(e) = wallet::send_update_notification(conn, &result.account.id).await {
        eprintln!("Error while communicating with APNS: {:?}", e);
    }

    Ok(HttpResponse::Ok().json(&result))
}
