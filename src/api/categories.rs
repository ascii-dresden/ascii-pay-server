use crate::api::utils::Search;
use crate::core::{fuzzy_vec_match, Category, Pool, ServiceResult};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct SearchCategory {
    #[serde(flatten)]
    pub category: Category,
    pub name_search: String,
    pub current_price_search: String,
}

impl SearchCategory {
    pub fn wrap(category: Category, search: &str) -> Option<SearchCategory> {
        let mut values = vec![category.name.clone()];

        values.push(
            category
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

        Some(SearchCategory {
            category,
            current_price_search: result.pop().expect(""),
            name_search: result.pop().expect(""),
        })
    }
}

/// GET route for `/categories`
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

/// GET route for `/category/{category_id}`
pub async fn get_category_edit(
    pool: web::Data<Pool>,
    category_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let category = Category::get(&conn, &Uuid::parse_str(&category_id)?)?;

    Ok(HttpResponse::Ok().json(&category))
}
