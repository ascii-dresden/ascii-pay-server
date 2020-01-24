pub mod accounts;
pub mod authentication;
pub mod categories;
pub mod products;
pub mod transactions;

use actix_web::web;

/// Setup routes for admin ui
pub fn init(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/api/v1")
            .service(
                web::resource("/barcode/find")
                    .route(web::get().to(authentication::get_barcode_find)),
            )
            .service(web::resource("/nfc/find").route(web::get().to(authentication::get_nfc_find)))
            // Setup account mangement related routes
            .service(web::resource("/accounts").route(web::get().to(accounts::get_accounts)))
            .service(
                web::resource("/account/{account_id}")
                    .route(web::get().to(accounts::get_account_edit)),
            )
            .service(
                web::resource("/transaction/token")
                    .route(web::post().to(transactions::post_transaction_token)),
            )
            .service(
                web::resource("/transaction/payment")
                    .route(web::post().to(transactions::post_transaction_payment)),
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
