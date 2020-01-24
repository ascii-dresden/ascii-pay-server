use crate::core::{
    authentication_barcode, authentication_nfc, Pool, Product, ServiceError, ServiceResult,
};
use actix_web::{web, HttpResponse};
use std::collections::HashMap;

/// GET route for `/api/v1/barcode/find`
pub async fn get_barcode_find(
    pool: web::Data<Pool>,
    data: web::Query<HashMap<String,String>>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    if !data.contains_key("code") {
        return Err(ServiceError::BadRequest("Parameter expected!", "A get parameter 'code' was expected!".to_owned()));
    }
    let code = data["code"].clone();

    if let Ok(product) = Product::get_by_barcode(&conn, &code) {
        return Ok(HttpResponse::Ok().json(&product));
    }

    if let Ok(account) = authentication_barcode::get(&conn, &code) {
        return Ok(HttpResponse::Ok().json(&account));
    }

    Err(ServiceError::NotFound)
}

/// GET route for `/api/v1/nfc/find`
pub async fn get_nfc_find(
    pool: web::Data<Pool>,
    id: web::Query<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let account = authentication_nfc::get(&conn, &id)?;

    Ok(HttpResponse::Ok().json(&account))
}
