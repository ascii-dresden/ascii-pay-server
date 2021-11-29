use std::ops::Deref;

use actix_web::{web, HttpResponse};
use chrono::NaiveDateTime;
use log::error;
use uuid::Uuid;

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
    let result = repo::transaction_payment(
        database_pool.deref(),
        redis_pool.deref(),
        &identity,
        input.into_inner(),
    )
    .await?;

    if let Err(e) = wallet::send_update_notification(database_pool.deref(), result.account.id).await
    {
        error!("Error while communicating with APNS: {:?}", e);
    }

    Ok(HttpResponse::Ok().json(&result))
}

#[derive(Debug, Deserialize)]
pub struct TransactionFilterInput {
    pub from: Option<NaiveDateTime>,
    pub to: Option<NaiveDateTime>,
}

/// GET route for `/api/v1/account/{account_id}/transactions`
pub async fn get_transactions_by_account(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
    transaction_filter: web::Query<TransactionFilterInput>,
) -> ServiceResult<HttpResponse> {
    let TransactionFilterInput { from, to } = transaction_filter.into_inner();
    let result = repo::get_transactions_by_account(
        database_pool.deref(),
        &identity,
        id.into_inner(),
        from,
        to,
    )
    .await?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/account/{account_id}/transaction/{transaction_id}`
pub async fn get_transaction_by_account(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<(Uuid, Uuid)>,
) -> ServiceResult<HttpResponse> {
    let (account_id, transaction_id) = id.into_inner();
    let result = repo::get_transaction_by_account(
        database_pool.deref(),
        &identity,
        account_id,
        transaction_id,
    )
    .await?;
    Ok(HttpResponse::Ok().json(&result))
}
