use crate::identity_service::Identity;
use crate::model::{Pool, ServiceResult};
use crate::repo::{self, AccountInput, AccountInputBarcode, AccountInputNfc};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

use super::Search;

/// GET route for `/api/v1/accounts`
pub async fn get_accounts(
    pool: web::Data<Pool>,
    identity: Identity,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::get_accounts(conn, &identity, query.search.as_deref())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// PUT route for `/api/v1/accounts`
pub async fn put_accounts(
    pool: web::Data<Pool>,
    identity: Identity,
    input: web::Json<AccountInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::create_account(conn, &identity, input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/account/{account_id}`
pub async fn get_account(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::get_account(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// POST route for `/api/v1/account/{account_id}`
pub async fn post_account(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
    input: web::Json<AccountInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::update_account(conn, &identity, id.into_inner(), input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/account/{account_id}`
pub async fn delete_account(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::delete_account(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// PUT route for `/api/v1/account/{account_id}/barcode`
pub async fn put_account_barcode(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
    input: web::Json<AccountInputBarcode>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::add_account_barcode(conn, &identity, id.into_inner(), input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/account/{account_id}/barcode`
pub async fn delete_account_barcode(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::delete_account_barcode(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// PUT route for `/api/v1/account/{account_id}/nfc`
pub async fn put_account_nfc(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
    input: web::Json<AccountInputNfc>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::add_account_nfc(conn, &identity, id.into_inner(), input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/account/{account_id}/nfc`
pub async fn delete_account_nfc(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::delete_account_nfc(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}
