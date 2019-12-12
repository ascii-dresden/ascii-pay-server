use crate::api::utils::Search;
use crate::core::{Category, Pool, Searchable, ServiceResult};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

/// GET route for `/categories`
pub async fn get_categories(
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let mut all_categories = Category::all(&conn)?;

    if let Some(search) = &query.search {
        let lower_search = search.trim().to_ascii_lowercase();
        all_categories = all_categories
            .into_iter()
            .filter(|a| a.contains(&lower_search))
            .collect();
    }

    Ok(HttpResponse::Ok().json(&all_categories))
}

/// GET route for `/category/{category_id}`
pub async fn get_category_edit(
    pool: web::Data<Pool>,
    category_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let category = Category::get(&conn, &Uuid::parse_str(&category_id)?)?;

    Ok(HttpResponse::Ok().json(&category))
}
