use crate::identity_service::Identity;
use crate::model::{Pool, ServiceResult};
use crate::repo::{self, ProductInput};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

use super::Search;

/// GET route for `/api/v1/products`
pub async fn get_products(
    pool: web::Data<Pool>,
    identity: Identity,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::get_products(conn, &identity, query.search.as_deref())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// PUT route for `/api/v1/products`
pub async fn put_products(
    identity: Identity,
    pool: web::Data<Pool>,
    input: web::Json<ProductInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::create_product(conn, &identity, input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/product/{product_id}`
pub async fn get_product(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::get_product(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// POST route for `/api/v1/product/{product_id}`
pub async fn post_product(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
    input: web::Json<ProductInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::update_product(conn, &identity, id.into_inner(), input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/product/{product_id}`
pub async fn delete_product(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::delete_product(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}
