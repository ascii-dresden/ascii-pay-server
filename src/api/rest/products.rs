use crate::identity_service::Identity;
use crate::repo;
use crate::utils::ServiceResult;
use actix_files::NamedFile;
use actix_web::{web, HttpRequest, HttpResponse};

/// GET route for `/api/v1/products`
pub async fn get_products(identity: Identity) -> ServiceResult<HttpResponse> {
    let result = repo::get_products(&identity)?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/product/{product_id}`
pub async fn get_product(identity: Identity, id: web::Path<String>) -> ServiceResult<HttpResponse> {
    let result = repo::get_product(&identity, &id.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// GET route for `/api/v1/product/{product_id}/image`
pub async fn get_product_image(
    req: HttpRequest,
    identity: Identity,
    id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let path = repo::get_product_image(&identity, &id.into_inner())?;

    Ok(NamedFile::open(path)?.into_response(&req)?)
}

/// GET route for `/api/v1/products/update`
pub async fn update_products(identity: Identity) -> ServiceResult<HttpResponse> {
    let result = repo::update_products(&identity)?;
    Ok(HttpResponse::Ok().json(&result))
}
