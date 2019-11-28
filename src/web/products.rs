use crate::core::{
    Category, DbConnection, Money, Pool, Product, Searchable, ServiceError, ServiceResult,
};
use crate::web::identity_policy::LoggedAccount;
use crate::web::utils::Search;
use actix_multipart::Multipart;
use actix_web::{http, web, HttpResponse};
use futures::prelude::*;
use handlebars::Handlebars;
use std::collections::HashMap;
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
pub async fn get_products(
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
pub async fn get_product_edit(
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
pub async fn post_product_edit(
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
pub async fn get_product_create(
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
pub async fn post_product_create(
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
pub async fn get_product_delete(
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
pub async fn get_product_remove_image(
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
pub async fn post_product_upload_image(
    pool: web::Data<Pool>,
    logged_account: LoggedAccount,
    product_id: web::Path<String>,
    multipart: Multipart,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;
    let mut product = Product::get(&conn, &product_id)?;
    let location = format!("/product/{}", &product_id);

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
            let data = chunk.unwrap();
            let mut pos = 0;
            while pos < data.len() {
                let bytes_written = file.write(&data[pos..])?;
                pos += bytes_written;
            }
        }
    }
    Ok(())
}
