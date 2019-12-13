mod categories;
mod products;
mod accounts;
pub mod utils;

use actix_web::web;

/// Setup routes for admin ui
pub fn init(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/api/v1")
            // Setup account mangement related routes
            .service(web::resource("/accounts").route(web::get().to(accounts::get_accounts)))
            .service(
                web::resource("/account/{account_id}")
                    .route(web::get().to(accounts::get_account_edit)),
            )
            // Setup product mangement related routes
            .service(web::resource("/products").route(web::get().to(products::get_products)))
            .service(
                web::resource("/product/{product_id}")
                    .route(web::get().to(products::get_product_edit)),
            )
            // Setup categories mangement related routes
            .service(web::resource("/categories").route(web::get().to(categories::get_categories)))
            .service(
                web::resource("/category/{category_id}")
                    .route(web::get().to(categories::get_category_edit)),
            ),
    );
}
