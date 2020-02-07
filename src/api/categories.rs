use crate::core::{Category, Pool, ServiceResult};
use crate::web::admin::categories::SearchCategory;
use crate::web::utils::Search;
use actix_web::{web, HttpResponse};
use uuid::Uuid;

/// GET route for `/api/v1/categories`
pub async fn get_categories(
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let search = match &query.search {
        Some(s) => s.clone(),
        None => "".to_owned(),
    };

    let lower_search = search.trim().to_ascii_lowercase();
    let search_categories: Vec<SearchCategory> = Category::all(&conn)?
        .into_iter()
        .filter_map(|c| SearchCategory::wrap(c, &lower_search))
        .collect();

    Ok(HttpResponse::Ok().json(&search_categories))
}

/// GET route for `/api/v1/category/{category_id}`
pub async fn get_category_edit(
    pool: web::Data<Pool>,
    category_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let category = Category::get(&conn, &Uuid::parse_str(&category_id)?)?;

    Ok(HttpResponse::Ok().json(&category))
}
