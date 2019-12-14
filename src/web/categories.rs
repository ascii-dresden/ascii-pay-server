use crate::core::{
    fuzzy_vec_match, Category, Money, Permission, Pool, ServiceError, ServiceResult,
};
use crate::login_required;
use crate::web::identity_policy::RetrievedAccount;
use crate::web::utils::{HbData, Search};
use actix_web::{http, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct FormCategory {
    pub id: String,
    pub name: String,
    #[serde(with = "crate::core::naive_date_time_serializer")]
    #[serde(rename = "price-date-create")]
    pub validity_start: chrono::NaiveDateTime,
    #[serde(rename = "price-value-create")]
    pub value: f32,
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

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
    hb: web::Data<Handlebars>,
    logged_account: RetrievedAccount,
    pool: web::Data<Pool>,
    query: web::Query<Search>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER);

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

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("search", &search)
        .with_data("categories", &search_categories)
        .render(&hb, "category_list")?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/category/{category_id}`
pub async fn get_category_edit(
    hb: web::Data<Handlebars>,
    logged_account: RetrievedAccount,
    pool: web::Data<Pool>,
    category_id: web::Path<String>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER);

    let conn = &pool.get()?;

    let category = Category::get(&conn, &Uuid::parse_str(&category_id)?)?;

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("category", &category)
        .render(&hb, "category_edit")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/category/{category_id}`
pub async fn post_category_edit(
    logged_account: RetrievedAccount,
    pool: web::Data<Pool>,
    category: web::Form<FormCategory>,
    category_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER);

    if *category_id != category.id {
        return Err(ServiceError::BadRequest(
            "Id missmage",
            "The category id of the url and the form do not match!".to_owned(),
        ));
    }

    let conn = &pool.get()?;

    let mut server_category = Category::get(&conn, &Uuid::parse_str(&category_id)?)?;

    server_category.name = category.name.clone();

    server_category.update(&conn)?;

    let mut delete_indeces = category
        .extra
        .keys()
        .filter_map(|k| k.trim_start_matches("delete-price-").parse::<usize>().ok())
        .collect::<Vec<usize>>();

    delete_indeces.sort_by(|a, b| b.cmp(a));

    for index in delete_indeces.iter() {
        server_category.remove_price(&conn, server_category.prices[*index].validity_start)?;
    }

    if category.value != 0.0 {
        server_category.add_price(
            &conn,
            category.validity_start,
            (category.value * 100.0) as Money,
        )?;
    }

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/categories")
        .finish())
}

/// GET route for `/category/create`
pub async fn get_category_create(
    hb: web::Data<Handlebars>,
    logged_account: RetrievedAccount,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER);

    let body = HbData::new(&request)
        .with_account(logged_account)
        .render(&hb, "category_create")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/category/create`
pub async fn post_category_create(
    logged_account: RetrievedAccount,
    pool: web::Data<Pool>,
    category: web::Form<FormCategory>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER);

    let conn = &pool.get()?;

    let mut server_category = Category::create(&conn, &category.name)?;

    if category.value != 0.0 {
        server_category.add_price(
            &conn,
            category.validity_start,
            (category.value * 100.0) as Money,
        )?;
    }

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/category/{}", server_category.id),
        )
        .finish())
}

/// GET route for `/category/delete/{category_id}`
pub async fn get_category_delete(
    _hb: web::Data<Handlebars>,
    logged_account: RetrievedAccount,
    _category_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER);

    println!("Delete is not supported!");

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/categories")
        .finish())
}
