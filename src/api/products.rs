use crate::core::{Pool, Product, ServiceResult};
use crate::web::admin::products::SearchProduct;
use crate::web::utils::Search;
use actix_web::{web, HttpResponse};
use uuid::Uuid;

/// GET route for `/api/v1/products`
pub async fn get_products(
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let search = match &query.search {
        Some(s) => s.clone(),
        None => "".to_owned(),
    };

    let lower_search = search.trim().to_ascii_lowercase();
    let search_products: Vec<SearchProduct> = Product::all(&conn)?
        .into_iter()
        .filter_map(|p| SearchProduct::wrap(p, &lower_search))
        .collect();

    Ok(HttpResponse::Ok().json(&search_products))
}

/// GET route for `/api/v1/product/{product_id}`
pub async fn get_product_edit(
    pool: web::Data<Pool>,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let product = Product::get(&conn, &Uuid::parse_str(&product_id)?)?;

    Ok(HttpResponse::Ok().json(&product))
}
