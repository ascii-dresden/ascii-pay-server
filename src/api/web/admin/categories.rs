use crate::api::web::utils::{HbData, IsJson, Search};
use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{
    fuzzy_vec_match, Category, Money, Permission, Pool, ServiceError, ServiceResult,
};
use actix_web::{http, web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use handlebars::Handlebars;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct FormCategory {
    pub id: String,
    pub name: String,
    #[serde(with = "crate::model::naive_date_time_serializer")]
    #[serde(rename = "price-date-create")]
    pub validity_start: NaiveDateTime,
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
        let values = vec![
            category.name.clone(),
            category
                .current_price
                .map(|v| format!("{:.2}€", (v as f32) / 100.0))
                .unwrap_or_else(|| "".to_owned()),
        ];

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

/// GET route for `/admin/categories`
pub async fn get_categories(
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    pool: web::Data<Pool>,
    query: web::Query<Search>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let account = if request.is_json() {
        identity.require_account(Permission::MEMBER)?
    } else {
        identity.require_account_with_redirect(Permission::MEMBER)?
    };

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

    if request.is_json() {
        Ok(HttpResponse::Ok().json(search_categories))
    } else {
        let body = HbData::new(&request)
            .with_account(account)
            .with_data("search", &search)
            .with_data("categories", &search_categories)
            .render(&hb, "admin_category_list")?;

        Ok(HttpResponse::Ok().body(body))
    }
}

/// GET route for `/admin/category/{category_id}`
pub async fn get_category_edit(
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    pool: web::Data<Pool>,
    category_id: web::Path<String>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let category = Category::get(&conn, &Uuid::parse_str(&category_id)?)?;

    let body = HbData::new(&request)
        .with_account(identity_account)
        .with_data("category", &category)
        .render(&hb, "admin_category_edit")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/admin/category/{category_id}`
pub async fn post_category_edit(
    identity: Identity,
    pool: web::Data<Pool>,
    category: web::Form<FormCategory>,
    category_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

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
        .header(http::header::LOCATION, "/admin/categories")
        .finish())
}

/// GET route for `/admin/category/create`
pub async fn get_category_create(
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::MEMBER)?;

    let body = HbData::new(&request)
        .with_account(identity_account)
        .render(&hb, "admin_category_create")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/admin/category/create`
pub async fn post_category_create(
    identity: Identity,
    pool: web::Data<Pool>,
    category: web::Form<FormCategory>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

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
            format!("/admin/category/{}", server_category.id),
        )
        .finish())
}

/// GET route for `/admin/category/delete/{category_id}`
pub async fn get_category_delete(
    _hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    _category_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    println!("Delete is not supported!");

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/admin/categories")
        .finish())
}
