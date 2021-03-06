pub mod accounts;
pub mod auth;
pub mod categories;
pub mod identification;
pub mod products;
pub mod transactions;

use actix_web::web;

/// Setup routes for admin ui
pub fn init(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/api/v1")
            .service(
                web::resource("/identify").route(web::post().to(identification::post_identify)),
            )
            .service(
                web::resource("/auth")
                    .route(web::get().to(auth::get_auth))
                    .route(web::post().to(auth::post_auth))
                    .route(web::delete().to(auth::delete_auth)),
            )
            // Setup account mangement related routes
            .service(
                web::resource("/accounts")
                    .route(web::get().to(accounts::get_accounts))
                    .route(web::put().to(accounts::put_accounts)),
            )
            .service(
                web::resource("/account/{account_id}/barcode")
                    .route(web::put().to(accounts::put_account_barcode))
                    .route(web::delete().to(accounts::delete_account_barcode)),
            )
            .service(
                web::resource("/account/{account_id}/nfc")
                    .route(web::put().to(accounts::put_account_nfc))
                    .route(web::delete().to(accounts::delete_account_nfc)),
            )
            .service(
                web::resource("/account/{account_id}")
                    .route(web::get().to(accounts::get_account))
                    .route(web::post().to(accounts::post_account))
                    .route(web::delete().to(accounts::delete_account)),
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
            .service(
                web::resource("/products")
                    .route(web::get().to(products::get_products))
                    .route(web::put().to(products::put_products)),
            )
            .service(
                web::resource("/product/{product_id}")
                    .route(web::get().to(products::get_product))
                    .route(web::post().to(products::post_product))
                    .route(web::delete().to(products::delete_product)),
            )
            // Setup categories mangement related routes
            .service(
                web::resource("/categories")
                    .route(web::get().to(categories::get_categories))
                    .route(web::put().to(categories::put_categories)),
            )
            .service(
                web::resource("/category/{category_id}")
                    .route(web::get().to(categories::get_category))
                    .route(web::post().to(categories::post_category))
                    .route(web::delete().to(categories::delete_category)),
            ),
    );
}
