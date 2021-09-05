use std::ops::DerefMut;

use actix_web::{web, HttpResponse};

use crate::{
    identity_service::Identity,
    model::wallet,
    repo::{self, PaymentInput},
    utils::{DatabasePool, RedisPool, ServiceResult},
};

/// POST route for `/api/v1/transaction/payment`
pub async fn post_transaction_payment(
    database_pool: web::Data<DatabasePool>,
    redis_pool: web::Data<RedisPool>,
    identity: Identity,
    input: web::Json<PaymentInput>,
) -> ServiceResult<HttpResponse> {
    let database_conn = &database_pool.get()?;
    let mut redis_conn = redis_pool.get()?;
    let result = repo::transaction_payment(
        database_conn,
        redis_conn.deref_mut(),
        &identity,
        input.into_inner(),
    )?;

    if let Err(e) = wallet::send_update_notification(database_conn, &result.account.id).await {
        eprintln!("Error while communicating with APNS: {:?}", e);
    }

    Ok(HttpResponse::Ok().json(&result))
}
