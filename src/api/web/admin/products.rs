use crate::api::web::utils::{HbData, IsJson, Search};
use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{
    fuzzy_vec_match, Category, DbConnection, Money, Permission, Pool, Product, ServiceError,
    ServiceResult,
};
use actix_multipart::Multipart;
use actix_web::{http, web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use futures::prelude::*;
use handlebars::Handlebars;
use std::collections::HashMap;
use std::io::Write;

use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct FormProduct {
    pub id: String,
    pub name: String,
    pub category: String,
    #[serde(with = "crate::model::naive_date_time_serializer")]
    #[serde(rename = "price-date-create")]
    pub validity_start: NaiveDateTime,
    #[serde(rename = "price-value-create")]
    pub value: f32,
    pub barcode: String,
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct SearchProduct {
    #[serde(flatten)]
    pub product: Product,
    pub name_search: String,
    pub category_search: String,
    pub current_price_search: String,
    pub barcode_search: String,
}

impl SearchProduct {
    pub fn wrap(product: Product, search: &str) -> Option<SearchProduct> {
        let values = vec![
            product.name.clone(),
            product
                .category
                .clone()
                .map(|v| v.name)
                .unwrap_or_else(|| "".to_owned()),
            product
                .current_price
                .map(|v| format!("{:.2}€", (v as f32) / 100.0))
                .unwrap_or_else(|| "".to_owned()),
            product.barcode.clone().unwrap_or_else(String::new),
        ];

        let mut result = if search.is_empty() {
            values
        } else {
            match fuzzy_vec_match(search, &values) {
                Some(r) => r,
                None => return None,
            }
        };

        Some(SearchProduct {
            product,
            barcode_search: result.pop().expect(""),
            current_price_search: result.pop().expect(""),
            category_search: result.pop().expect(""),
            name_search: result.pop().expect(""),
        })
    }
}

/// GET route for `/admin/products`
pub async fn get_products(
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    pool: web::Data<Pool>,
    query: web::Query<Search>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let identity_account = if request.is_json() {
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
    let search_products: Vec<SearchProduct> = Product::all(&conn)?
        .into_iter()
        .filter_map(|p| SearchProduct::wrap(p, &lower_search))
        .collect();

    if request.is_json() {
        Ok(HttpResponse::Ok().json(search_products))
    } else {
        let body = HbData::new(&request)
            .with_account(identity_account)
            .with_data("search", &search)
            .with_data("products", &search_products)
            .render(&hb, "admin_product_list")?;

        Ok(HttpResponse::Ok().body(body))
    }
}

/// GET route for `/admin/product/{product_id}`
pub async fn get_product_edit(
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    pool: web::Data<Pool>,
    product_id: web::Path<String>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let product = Product::get(&conn, &Uuid::parse_str(&product_id)?)?;

    let all_categories = Category::all(&conn)?;

    let body = HbData::new(&request)
        .with_account(identity_account)
        .with_data("product", &product)
        .with_data("categories", &all_categories)
        .render(&hb, "admin_product_edit")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/admin/product/{product_id}`
pub async fn post_product_edit(
    identity: Identity,
    pool: web::Data<Pool>,
    product: web::Form<FormProduct>,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    if *product_id != product.id {
        return Err(ServiceError::BadRequest(
            "Id missmage",
            "The product id of the url and the form do not match!".to_owned(),
        ));
    }

    let conn = &pool.get()?;

    let mut server_product = Product::get(&conn, &Uuid::parse_str(&product_id)?)?;

    let category = if product.category.is_empty() {
        None
    } else {
        Some(Category::get(&conn, &Uuid::parse_str(&product.category)?)?)
    };

    server_product.name = product.name.clone();
    server_product.category = category;

    server_product.barcode = if product.barcode.trim().is_empty() {
        None
    } else {
        Some(product.barcode.trim().to_owned())
    };

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
        .header(http::header::LOCATION, "/admin/products")
        .finish())
}

/// GET route for `/admin/product/create`
pub async fn get_product_create(
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    pool: web::Data<Pool>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::MEMBER)?;
    let conn = &pool.get()?;

    let all_categories = Category::all(&conn)?;

    let body = HbData::new(&request)
        .with_account(identity_account)
        .with_data("categories", &all_categories)
        .render(&hb, "admin_product_create")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/admin/product/create`
pub async fn post_product_create(
    identity: Identity,
    pool: web::Data<Pool>,
    product: web::Form<FormProduct>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let category = if product.category.is_empty() {
        None
    } else {
        Some(Category::get(&conn, &Uuid::parse_str(&product.category)?)?)
    };

    let mut server_product = Product::create(&conn, &product.name, category)?;

    if product.value != 0.0 {
        server_product.add_price(
            &conn,
            product.validity_start,
            (product.value * 100.0) as Money,
        )?;
    }

    server_product.barcode = if product.barcode.trim().is_empty() {
        None
    } else {
        Some(product.barcode.trim().to_owned())
    };

    server_product.update(&conn)?;

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/admin/product/{}", server_product.id),
        )
        .finish())
}

/// GET route for `/admin/product/delete/{product_id}`
pub async fn get_product_delete(
    _hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    _product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    println!("Delete is not supported!");

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/admin/products")
        .finish())
}

/// GET route for `/admin/product/remove-image/{product_id}`
pub async fn get_product_remove_image(
    pool: web::Data<Pool>,
    identity: Identity,
    product_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let mut product = Product::get(&conn, &Uuid::parse_str(&product_id)?)?;

    product.remove_image(&conn)?;

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/admin/product/{}", &product_id),
        )
        .finish())
}

/// POST route for `/admin/product/upload-image/{product_id}`
pub async fn post_product_upload_image(
    pool: web::Data<Pool>,
    identity: Identity,
    product_id: web::Path<String>,
    multipart: Multipart,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    let conn = &pool.get()?;
    let mut product = Product::get(&conn, &Uuid::parse_str(&product_id)?)?;
    let location = format!("/admin/product/{}", &product_id);

    save_file(multipart, &conn, &mut product).await?;
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, location)
        .finish())
}

const ALLOWED_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "svg"];

/// Read the multipart stream and save content to file
async fn save_file(
    mut payload: Multipart,
    conn: &DbConnection,
    product: &mut Product,
) -> ServiceResult<()> {
    // iterate over multipart stream
    while let Some(item) = payload.next().await {
        let mut field = item?;

        // verify the file content type
        let file_extension = field
            .content_type()
            .subtype()
            .as_str()
            .to_ascii_lowercase()
            .to_owned();

        if !ALLOWED_EXTENSIONS.iter().any(|s| s == &file_extension) {
            return Err(ServiceError::InternalServerError(
                "Unsupported",
                "".to_owned(),
            ));
        }

        let mut file = product.set_image(&conn, &file_extension)?;

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            let mut pos = 0;
            while pos < data.len() {
                let bytes_written = file.write(&data[pos..])?;
                pos += bytes_written;
            }
        }

        file.flush()?;
        println!("Flush file!")
    }
    Ok(())
}
