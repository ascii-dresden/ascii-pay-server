use crate::identity_service::Identity;
use crate::repo::{self, ProductInput};
use crate::utils::{DatabasePool, ServiceResult};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

use super::Search;

/// GET route for `/api/v1/products`
pub async fn get_products(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let database_conn = &database_pool.get()?;
    let result = repo::get_products(database_conn, &identity, query.search.as_deref())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// PUT route for `/api/v1/products`
pub async fn put_products(
    identity: Identity,
    database_pool: web::Data<DatabasePool>,
    input: web::Json<ProductInput>,
) -> ServiceResult<HttpResponse> {
    let database_conn = &database_pool.get()?;
    let result = repo::create_product(database_conn, &identity, input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/product/{product_id}`
pub async fn get_product(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let database_conn = &database_pool.get()?;
    let result = repo::get_product(database_conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// POST route for `/api/v1/product/{product_id}`
pub async fn post_product(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
    input: web::Json<ProductInput>,
) -> ServiceResult<HttpResponse> {
    let database_conn = &database_pool.get()?;
    let result = repo::update_product(
        database_conn,
        &identity,
        id.into_inner(),
        input.into_inner(),
    )?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/product/{product_id}`
pub async fn delete_product(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let database_conn = &database_pool.get()?;
    let result = repo::delete_product(database_conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}
