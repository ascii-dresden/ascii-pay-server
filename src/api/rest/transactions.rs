use std::ops::DerefMut;

use actix_web::{web, HttpResponse};
use uuid::Uuid;

use crate::{
    identity_service::Identity,
    model::wallet,
    repo::{self, PaymentInput, TransactionFilterInput},
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

    if let Err(e) = wallet::send_update_notification(database_conn, result.account.id).await {
        eprintln!("Error while communicating with APNS: {:?}", e);
    }

    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/account/{account_id}/transactions`
pub async fn get_transactions_by_account(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
    transaction_filter: web::Query<TransactionFilterInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &database_pool.get()?;
    let result = repo::get_transactions_by_account(
        conn,
        &identity,
        id.into_inner(),
        Some(transaction_filter.into_inner()),
    )?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/account/{account_id}/transaction/{transaction_id}`
pub async fn get_transaction_by_account(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<(Uuid, Uuid)>,
) -> ServiceResult<HttpResponse> {
    let conn = &database_pool.get()?;

    let (account_id, transaction_id) = id.into_inner();
    let result = repo::get_transaction_by_account(conn, &identity, account_id, transaction_id)?;
    Ok(HttpResponse::Ok().json(&result))
}
