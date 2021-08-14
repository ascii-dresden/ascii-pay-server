use crate::core::{Category, Permission, Pool, Product, ServiceError, ServiceResult};
use crate::identity_service::{Identity, IdentityRequire};
use crate::web::admin::products::SearchProduct;
use crate::web::utils::Search;
use actix_web::{web, HttpResponse};
use uuid::Uuid;

/// GET route for `/api/v1/products`
pub async fn get_products(
    pool: web::Data<Pool>,
    identity: Identity,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;
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

/// PUT route for `/api/v1/products`
pub async fn put_products(
    identity: Identity,
    pool: web::Data<Pool>,
    product: web::Json<Product>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;
    let conn = &pool.get()?;

    let category = if let Some(x) = &product.category {
        Some(Category::get(&conn, &x.id)?)
    } else {
        None
    };

    let mut server_product = Product::create(&conn, &product.name, category)?;

    server_product.barcode = product.barcode.clone();
    server_product.update(&conn)?;

    server_product.update_prices(&conn, &product.prices)?;

    Ok(HttpResponse::Created().json(json!({
        "id": server_product.id
    })))
}

/// GET route for `/api/v1/product/{product_id}`
pub async fn get_product(
    pool: web::Data<Pool>,
    identity: Identity,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;
    let conn = &pool.get()?;
    let product = Product::get(&conn, &Uuid::parse_str(&product_id)?)?;

    Ok(HttpResponse::Ok().json(&product))
}

/// POST route for `/api/v1/product/{product_id}`
pub async fn post_product(
    identity: Identity,
    pool: web::Data<Pool>,
    product: web::Json<Product>,
    product_id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    if *product_id != product.id {
        return Err(ServiceError::BadRequest(
            "Id missmage",
            "The product id of the url and the json do not match!".to_owned(),
        ));
    }

    let conn = &pool.get()?;

    let mut server_product = Product::get(&conn, &product_id)?;

    let category = if let Some(x) = &product.category {
        Some(Category::get(&conn, &x.id)?)
    } else {
        None
    };

    server_product.name = product.name.clone();
    server_product.barcode = product.barcode.clone();
    server_product.category = category;

    server_product.update(&conn)?;

    server_product.update_prices(&conn, &product.prices)?;

    Ok(HttpResponse::Ok().finish())
}

/// DELETE route for `/api/v1/product/{product_id}`
pub async fn delete_product(
    identity: Identity,
    _product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    println!("Delete is not supported!");

    Ok(HttpResponse::MethodNotAllowed().finish())
}
