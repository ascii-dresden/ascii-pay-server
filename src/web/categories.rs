use crate::core::{Category, Money, Pool, Searchable, ServiceError, ServiceResult};
use crate::login_required;
use crate::web::identity_policy::{RetrievedAccount};
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

/// GET route for `/categories`
pub async fn get_categories(
    hb: web::Data<Handlebars>,
    logged_account: RetrievedAccount,
    pool: web::Data<Pool>,
    query: web::Query<Search>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account);

    let conn = &pool.get()?;

    let mut all_categories = Category::all(&conn)?;

    let search = if let Some(search) = &query.search {
        let lower_search = search.trim().to_ascii_lowercase();
        all_categories = all_categories
            .into_iter()
            .filter(|a| a.contains(&lower_search))
            .collect();
        search.clone()
    } else {
        "".to_owned()
    };

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("search", &search)
        .with_data("categories", &all_categories)
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
    let logged_account = login_required!(logged_account);

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
    let _logged_account = login_required!(logged_account);

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
    let logged_account = login_required!(logged_account);

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
    let _logged_account = login_required!(logged_account);

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
    let _logged_account = login_required!(logged_account);

    println!("Delete is not supported!");

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/categories")
        .finish())
}
