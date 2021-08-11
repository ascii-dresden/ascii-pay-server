use crate::core::{Category, Permission, Pool, ServiceError, ServiceResult};
use crate::identity_policy::{Action, RetrievedAccount};
use crate::login_or_client_cert_required;
use crate::web::admin::categories::SearchCategory;
use crate::web::utils::Search;
use actix_web::{web, HttpRequest, HttpResponse};
use uuid::Uuid;

/// GET route for `/api/v1/categories`
pub async fn get_categories(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    query: web::Query<Search>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_or_client_cert_required!(
        request,
        logged_account,
        Permission::MEMBER,
        Action::FORBIDDEN
    );

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

/// PUT route for `/api/v1/categories`
pub async fn put_categories(
    logged_account: RetrievedAccount,
    pool: web::Data<Pool>,
    category: web::Json<Category>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_or_client_cert_required!(
        request,
        logged_account,
        Permission::MEMBER,
        Action::FORBIDDEN
    );

    let conn = &pool.get()?;

    let mut server_category = Category::create(&conn, &category.name)?;

    server_category.update_prices(&conn, &category.prices)?;

    Ok(HttpResponse::Created().json(json!({
        "id": server_category.id
    })))
}

/// GET route for `/api/v1/category/{category_id}`
pub async fn get_category(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    category_id: web::Path<String>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_or_client_cert_required!(
        request,
        logged_account,
        Permission::MEMBER,
        Action::FORBIDDEN
    );

    let conn = &pool.get()?;

    let category = Category::get(&conn, &Uuid::parse_str(&category_id)?)?;

    Ok(HttpResponse::Ok().json(&category))
}

/// POST route for `/api/v1/category/{category_id}`
pub async fn post_category(
    logged_account: RetrievedAccount,
    pool: web::Data<Pool>,
    category: web::Json<Category>,
    category_id: web::Path<Uuid>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_or_client_cert_required!(
        request,
        logged_account,
        Permission::MEMBER,
        Action::FORBIDDEN
    );

    if *category_id != category.id {
        return Err(ServiceError::BadRequest(
            "Id missmage",
            "The category id of the url and the json do not match!".to_owned(),
        ));
    }

    let conn = &pool.get()?;

    let mut server_category = Category::get(&conn, &category_id)?;

    server_category.name = category.name.clone();
    server_category.update(&conn)?;

    server_category.update_prices(&conn, &category.prices)?;

    Ok(HttpResponse::Ok().finish())
}

/// DELETE route for `/api/v1/category/{category_id}`
pub async fn delete_category(
    logged_account: RetrievedAccount,
    _category_id: web::Path<String>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_or_client_cert_required!(
        request,
        logged_account,
        Permission::MEMBER,
        Action::FORBIDDEN
    );

    println!("Delete is not supported!");

    Ok(HttpResponse::MethodNotAllowed().finish())
}
