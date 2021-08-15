use crate::identity_service::Identity;
use crate::model::{Pool, ServiceResult};
use crate::repo::{self, CategoryInput};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

use super::Search;

/// GET route for `/api/v1/categories`
pub async fn get_categories(
    pool: web::Data<Pool>,
    identity: Identity,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::get_categories(conn, &identity, query.search.as_deref())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// PUT route for `/api/v1/categories`
pub async fn put_categories(
    pool: web::Data<Pool>,
    identity: Identity,
    input: web::Json<CategoryInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::create_category(conn, &identity, input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/category/{category_id}`
pub async fn get_category(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::get_category(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// POST route for `/api/v1/category/{category_id}`
pub async fn post_category(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
    input: web::Json<CategoryInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::update_category(conn, &identity, id.into_inner(), input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/category/{category_id}`
pub async fn delete_category(
    pool: web::Data<Pool>,
    identity: Identity,
    id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::delete_category(conn, &identity, id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}
