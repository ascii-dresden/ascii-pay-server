use crate::identity_service::Identity;
use crate::repo::{self, AccountCreateInput, AccountUpdateInput};
use crate::utils::{DatabasePool, RedisPool, ServiceResult};
use actix_web::{web, HttpResponse};
use lazy_static::__Deref;
use uuid::Uuid;

use super::Search;

/// GET route for `/api/v1/accounts`
pub async fn get_accounts(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let result =
        repo::get_accounts(database_pool.deref(), &identity, query.search.as_deref()).await?;
    Ok(HttpResponse::Ok().json(&result))
}

/// PUT route for `/api/v1/accounts`
pub async fn put_accounts(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    input: web::Json<AccountCreateInput>,
) -> ServiceResult<HttpResponse> {
    let result = repo::create_account(database_pool.deref(), &identity, input.into_inner()).await?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/account/{account_id}`
pub async fn get_account(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let result = repo::get_account(database_pool.deref(), &identity, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(&result))
}

/// POST route for `/api/v1/account/{account_id}`
pub async fn post_account(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
    input: web::Json<AccountUpdateInput>,
) -> ServiceResult<HttpResponse> {
    let result = repo::update_account(
        database_pool.deref(),
        &identity,
        id.into_inner(),
        input.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/account/{account_id}`
pub async fn delete_account(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let result = repo::delete_account(database_pool.deref(), &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/account/{account_id}/nfc/{card_id}`
pub async fn delete_account_nfc(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    path: web::Path<(Uuid, String)>,
) -> ServiceResult<HttpResponse> {
    let (account_id, card_id) = path.into_inner();
    let result =
        repo::authenticate_nfc_delete_card(database_pool.deref(), &identity, account_id, &card_id)
            .await?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/account/{account_id}/access-token`
pub async fn get_account_access_token(
    database_pool: web::Data<DatabasePool>,
    redis_pool: web::Data<RedisPool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let result = repo::authenticate_account(
        database_pool.deref(),
        redis_pool.deref(),
        &identity,
        id.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(&result))
}
