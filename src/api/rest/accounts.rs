use std::ops::DerefMut;

use crate::identity_service::Identity;
use crate::repo::{self, AccountCreateInput, AccountUpdateInput};
use crate::utils::{DatabasePool, RedisPool, ServiceResult};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

use super::Search;

/// GET route for `/api/v1/accounts`
pub async fn get_accounts(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &database_pool.get()?;
    let result = repo::get_accounts(conn, &identity, query.search.as_deref())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// PUT route for `/api/v1/accounts`
pub async fn put_accounts(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    input: web::Json<AccountCreateInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &database_pool.get()?;
    let result = repo::create_account(conn, &identity, input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/account/{account_id}`
pub async fn get_account(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &database_pool.get()?;
    let result = repo::get_account(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// POST route for `/api/v1/account/{account_id}`
pub async fn post_account(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
    input: web::Json<AccountUpdateInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &database_pool.get()?;
    let result = repo::update_account(conn, &identity, id.into_inner(), input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/account/{account_id}`
pub async fn delete_account(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &database_pool.get()?;
    let result = repo::delete_account(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/account/{account_id}/nfc`
pub async fn delete_account_nfc(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &database_pool.get()?;
    let result = repo::authenticate_nfc_delete_card(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/account/{account_id}/access-token`
pub async fn get_account_access_token(
    database_pool: web::Data<DatabasePool>,
    redis_pool: web::Data<RedisPool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let database_conn = &database_pool.get()?;
    let mut redis_conn = redis_pool.get()?;
    let result = repo::authenticate_account(
        database_conn,
        redis_conn.deref_mut(),
        &identity,
        id.into_inner(),
    )?;
    Ok(HttpResponse::Ok().json(&result))
}
