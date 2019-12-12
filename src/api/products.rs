use crate::api::utils::Search;
use crate::core::{Pool, Product, Searchable, ServiceResult};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

/// GET route for `/products`
pub async fn get_products(
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let mut all_products = Product::all(&conn)?;

    if let Some(search) = &query.search {
        let lower_search = search.trim().to_ascii_lowercase();
        all_products = all_products
            .into_iter()
            .filter(|a| a.contains(&lower_search))
            .collect();
    }

    Ok(HttpResponse::Ok().json(&all_products))
}

/// GET route for `/product/{product_id}`
pub async fn get_product_edit(
    pool: web::Data<Pool>,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let product = Product::get(&conn, &Uuid::parse_str(&product_id)?)?;

    Ok(HttpResponse::Ok().json(&product))
}
