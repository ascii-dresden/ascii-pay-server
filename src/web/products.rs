use actix_web::{http, web, HttpResponse};
use handlebars::Handlebars;
use std::collections::HashMap;

use crate::core::{Money, Pool, Product, ServiceResult};
use crate::web::utils::{LoggedAccount, Search};

#[derive(Debug, Serialize, Deserialize)]
pub struct FormProduct {
    pub id: String,
    pub name: String,
    pub category: String,
    #[serde(with = "crate::core::naive_date_time_serializer")]
    #[serde(rename = "price-date-create")]
    pub validity_start: chrono::NaiveDateTime,
    #[serde(rename = "price-value-create")]
    pub value: f32,
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

/// GET route for `/products`
pub fn get_products(
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let mut all_products = Product::all(&conn)?;

    let search = if let Some(search) = &query.search {
        let lower_search = search.to_ascii_lowercase();
        all_products = all_products
            .into_iter()
            .filter(|a| a.name.to_ascii_lowercase().contains(&lower_search))
            .collect();
        search.clone()
    } else {
        "".to_owned()
    };

    let data = json!({
        "search": search,
        "products": all_products
    });

    let body = hb.render("product_list", &data)?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/product/{product_id}`
pub fn get_product_edit(
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
    pool: web::Data<Pool>,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let product = Product::get(&conn, &product_id)?;

    let body = hb.render("product_edit", &product)?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/product/{product_id}`
pub fn post_product_edit(
    _: LoggedAccount,
    pool: web::Data<Pool>,
    product: web::Form<FormProduct>,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    if *product_id != product.id {
        panic!("at the disco");
    }

    let conn = &pool.get()?;
    let mut server_product = Product::get(&conn, &product_id)?;

    server_product.name = product.name.clone();
    server_product.category = product.category.clone();

    server_product.update(&conn)?;

    let mut delete_indeces = product
        .extra
        .keys()
        .filter_map(|k| k.trim_start_matches("delete-price-").parse::<usize>().ok())
        .collect::<Vec<usize>>();

    delete_indeces.sort_by(|a, b| b.cmp(a));

    for index in delete_indeces.iter() {
        server_product.remove_price(&conn, server_product.prices[*index].validity_start)?;
    }

    if product.value != 0.0 {
        server_product.add_price(
            &conn,
            product.validity_start,
            (product.value * 100.0) as Money,
        )?;
    }

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/products")
        .finish())
}

/// GET route for `/product/create`
pub fn get_product_create(
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
) -> ServiceResult<HttpResponse> {
    let body = hb.render("product_create", &false)?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/product/create`
pub fn post_product_create(
    _: LoggedAccount,
    pool: web::Data<Pool>,
    product: web::Form<FormProduct>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let mut server_product = Product::create(&conn, &product.name, &product.category)?;

    if product.value != 0.0 {
        server_product.add_price(
            &conn,
            product.validity_start,
            (product.value * 100.0) as Money,
        )?;
    }

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/products")
        .finish())
}

/// GET route for `/product/delete/{product_id}`
pub fn get_product_delete(
    _hb: web::Data<Handlebars>,
    _: LoggedAccount,
    _pool: web::Data<Pool>,
    _product_id: web::Path<String>,
) -> HttpResponse {
    println!("Delete is not supported!");

    HttpResponse::Found()
        .header(http::header::LOCATION, "/products")
        .finish()
}
