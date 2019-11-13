mod accounts;
mod index;
mod products;
mod utils;

use actix_files as fs;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::web;

// Encryption key for cookies
lazy_static::lazy_static! {
static ref SECRET_KEY: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));
}

/// Setup routes for admin ui
pub fn init(config: &mut web::ServiceConfig) {
    let domain = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());

    config.service(
        web::scope("/")
            // Set identity service for encrypted cookies
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(SECRET_KEY.as_bytes())
                    .name("auth")
                    .path("/")
                    .domain(&domain)
                    .max_age_time(chrono::Duration::days(1))
                    .secure(false),
            ))
            // Setup static routes
            .service(fs::Files::new("/stylesheets", "static/stylesheets/"))
            .service(fs::Files::new("/images", "static/images/"))
            // Setup index/login routes
            .service(web::resource("").route(web::get().to(index::get_index)))
            .service(web::resource("/login").route(web::post().to(index::post_index_login)))
            .service(web::resource("/logout").route(web::get().to(index::get_logout)))
            // Setup account mangement related routes
            .service(web::resource("/accounts").route(web::get().to(accounts::get_accounts)))
            .service(
                web::resource("/account/create")
                    .route(web::post().to(accounts::post_account_create))
                    .route(web::get().to(accounts::get_account_create)),
            )
            .service(
                web::resource("/account/delete/{account_id}")
                    .route(web::get().to(accounts::delete_get)),
            )
            .service(
                web::resource("/account/{account_id}")
                    .route(web::post().to(accounts::post_account_edit))
                    .route(web::get().to(accounts::get_account_edit)),
            )
            // Setup product mangement related routes
            .service(web::resource("/products").route(web::get().to(products::get_products)))
            .service(
                web::resource("/product/create")
                    .route(web::post().to(products::post_product_create))
                    .route(web::get().to(products::get_product_create)),
            )
            .service(
                web::resource("/product/delete/{product_id}")
                    .route(web::get().to(products::get_product_delete)),
            )
            .service(
                web::resource("/product/{product_id}")
                    .route(web::post().to(products::post_product_edit))
                    .route(web::get().to(products::get_product_edit)),
            ),
    );
}
