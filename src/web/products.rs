use actix_web::{http, web, HttpResponse};
use handlebars::Handlebars;
use std::collections::HashMap;

use crate::core::{Money, Pool, Product};
use crate::web::{LoggedAccount, WebResult};

#[derive(Deserialize)]
pub struct Search {
    search: Option<String>,
}

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

pub fn list(
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> WebResult<HttpResponse> {
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

pub fn edit_get(
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
    pool: web::Data<Pool>,
    product_id: web::Path<String>,
) -> WebResult<HttpResponse> {
    let conn = &pool.get()?;
    let product = Product::get(&conn, &product_id)?;

    let body = hb.render("product_edit", &product)?;

    Ok(HttpResponse::Ok().body(body))
}

pub fn edit_post(
    _: LoggedAccount,
    pool: web::Data<Pool>,
    product: web::Form<FormProduct>,
    product_id: web::Path<String>,
) -> WebResult<HttpResponse> {
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
        .into_iter()
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

pub fn create_get(hb: web::Data<Handlebars>, _: LoggedAccount) -> WebResult<HttpResponse> {
    let body = hb.render("product_create", &false)?;

    Ok(HttpResponse::Ok().body(body))
}

pub fn create_post(
    _: LoggedAccount,
    pool: web::Data<Pool>,
    product: web::Form<FormProduct>,
) -> WebResult<HttpResponse> {
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

pub fn delete_get(
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

trait EmptyToNone<T> {
    fn empty_to_none(&self) -> Option<T>;
}

impl EmptyToNone<String> for Option<String> {
    fn empty_to_none(&self) -> Option<String> {
        match self {
            None => None,
            Some(s) => {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }
        }
    }
}
