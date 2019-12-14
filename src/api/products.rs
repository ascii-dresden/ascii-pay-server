use crate::api::utils::Search;
use crate::core::{fuzzy_vec_match, Pool, Product, ServiceResult};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct SearchProduct {
    #[serde(flatten)]
    pub product: Product,
    pub name_search: String,
    pub category_search: String,
    pub current_price_search: String,
}

impl SearchProduct {
    pub fn wrap(product: Product, search: &str) -> Option<SearchProduct> {
        let mut values = vec![product.name.clone()];

        values.push(
            product
                .category
                .clone()
                .map(|v| v.name)
                .unwrap_or_else(|| "".to_owned()),
        );

        values.push(
            product
                .current_price
                .map(|v| format!("{:.2}€", (v as f32) / 100.0))
                .unwrap_or_else(|| "".to_owned()),
        );

        let mut result = if search.is_empty() {
            values
        } else {
            match fuzzy_vec_match(search, &values) {
                Some(r) => r,
                None => return None,
            }
        };

        Some(SearchProduct {
            product,
            current_price_search: result.pop().expect(""),
            category_search: result.pop().expect(""),
            name_search: result.pop().expect(""),
        })
    }
}

/// GET route for `/products`
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

/// GET route for `/product/{product_id}`
pub async fn get_product_edit(
    pool: web::Data<Pool>,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let product = Product::get(&conn, &Uuid::parse_str(&product_id)?)?;

    Ok(HttpResponse::Ok().json(&product))
}
