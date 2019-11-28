use actix_web::{error, http, web, HttpResponse};
use handlebars::Handlebars;
use std::collections::HashMap;

use crate::core::{
    Category, DbConnection, Money, Pool, Product, Searchable, ServiceError, ServiceResult,
};
use crate::web::identity_policy::LoggedAccount;
use crate::web::utils::Search;
use actix_multipart::{Field, Multipart, MultipartError};
use futures::future::{err, Either};
use futures::prelude::*;
use std::io::Write;

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
    logged_account: LoggedAccount,
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;

    let mut all_products = Product::all(&conn)?;

    let search = if let Some(search) = &query.search {
        let lower_search = search.trim().to_ascii_lowercase();
        all_products = all_products
            .into_iter()
            .filter(|a| a.contains(&lower_search))
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
    logged_account: LoggedAccount,
    pool: web::Data<Pool>,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;

    let product = Product::get(&conn, &product_id)?;

    let all_categories = Category::all(&conn)?;
    let body = hb.render(
        "product_edit",
        &json!({
            "product": &product,
            "categories": &all_categories
        }),
    )?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/product/{product_id}`
pub fn post_product_edit(
    logged_account: LoggedAccount,
    pool: web::Data<Pool>,
    product: web::Form<FormProduct>,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    if *product_id != product.id {
        return Err(ServiceError::BadRequest(
            "Id missmage",
            "The product id of the url and the form do not match!".to_owned(),
        ));
    }

    let conn = &pool.get()?;

    let mut server_product = Product::get(&conn, &product_id)?;

    let category = if product.category == "" {
        None
    } else {
        Some(Category::get(&conn, &product.category)?)
    };

    server_product.name = product.name.clone();
    server_product.category = category;

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
    logged_account: LoggedAccount,
    pool: web::Data<Pool>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;
    let conn = &pool.get()?;

    let all_categories = Category::all(&conn)?;
    let body = hb.render("product_create", &json!({ "categories": &all_categories }))?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/product/create`
pub fn post_product_create(
    logged_account: LoggedAccount,
    pool: web::Data<Pool>,
    product: web::Form<FormProduct>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;

    let category = if product.category == "" {
        None
    } else {
        Some(Category::get(&conn, &product.category)?)
    };

    let mut server_product = Product::create(&conn, &product.name, category)?;

    if product.value != 0.0 {
        server_product.add_price(
            &conn,
            product.validity_start,
            (product.value * 100.0) as Money,
        )?;
    }

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/product/{}", server_product.id),
        )
        .finish())
}

/// GET route for `/product/delete/{product_id}`
pub fn get_product_delete(
    _hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
    _product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    println!("Delete is not supported!");

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/products")
        .finish())
}

/// GET route for `/product/remove-image/{product_id}`
pub fn get_product_remove_image(
    pool: web::Data<Pool>,
    logged_account: LoggedAccount,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;

    let mut product = Product::get(&conn, &product_id)?;

    product.remove_image(&conn)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, format!("/product/{}", &product_id))
        .finish())
}

/// POST route for `/product/upload-image/{product_id}`
pub fn post_product_upload_image(
    pool: web::Data<Pool>,
    logged_account: LoggedAccount,
    product_id: web::Path<String>,
    multipart: Multipart,
) -> impl Future<Output = ServiceResult<HttpResponse>> {
    logged_account.require_member().unwrap();

    let mut product = Product::get(&pool.clone().get().unwrap(), &product_id).unwrap();
    let location = format!("/product/{}", &product_id);

    multipart
        .map_err(error::ErrorInternalServerError)
        .map(move |field| save_file(field, pool.get().unwrap(), &mut product).into_stream())
        .flatten()
        .collect()
        .map(|_sizes| {
            // println!("###: {:?}", sizes);
            HttpResponse::Found()
                .header(http::header::LOCATION, location)
                .finish()
        })
        .map_err(|e| {
            println!("--- failed: {}", e);
            e
        })
}

const ALLOWED_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "svg"];

/// Read the multipart stream and save content to file
fn save_file(
    field: Field,
    conn: r2d2::PooledConnection<diesel::r2d2::ConnectionManager<DbConnection>>,
    product: &mut Product,
) -> impl Future<Output = ServiceResult<i64>> {
    let file_extension = field
        .content_type()
        .subtype()
        .as_str()
        .to_ascii_lowercase()
        .to_owned();

    if !ALLOWED_EXTENSIONS.iter().any(|s| s == &file_extension) {
        return Either::Left(err(error::ErrorInternalServerError(
            ServiceError::InternalServerError("Unsupported", "".to_owned()),
        )));
    }

    let file = match product.set_image(&conn, &file_extension) {
        Ok(file) => file,
        Err(e) => return Either::Left(err(error::ErrorInternalServerError(e))),
    };

    Either::Right(
        field
            .fold((file, 0i64), move |(mut file, mut acc), bytes| {
                // fs operations are blocking, we have to execute writes
                // on threadpool
                web::block(move || {
                    let bytes = bytes?;
                    file.write_all(bytes.as_ref()).map_err(|e| {
                        println!("file.write_all failed: {:?}", e);
                        MultipartError::Payload(actix_web::error::PayloadError::Io(e))
                    })?;
                    acc += bytes.len() as i64;
                    Ok((file, acc))
                })
                // .map_err(|e: error::BlockingError<MultipartError>| match e {
                //     error::BlockingError::Error(e) => e,
                //     error::BlockingError::Canceled => MultipartError::Incomplete,
                // })
            })
            .map(|(_, acc)| acc)
            .map_err(|e| {
                println!("save_file failed, {:?}", e);
                error::ErrorInternalServerError(e)
            }),
    )
}
