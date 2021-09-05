pub mod accounts;
pub mod auth;
pub mod categories;
pub mod products;
pub mod transactions;

use actix_web::web;

/// Helper to deserialize search queries
#[derive(Debug, Deserialize)]
pub struct Search {
    pub search: Option<String>,
}

pub fn init(config: &mut web::ServiceConfig) {
    config
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
            web::resource("/account/{account_id}/nfc")
                .route(web::delete().to(accounts::delete_account_nfc)),
        )
        .service(
            web::resource("/account/{account_id}/access-token")
                .route(web::get().to(accounts::get_account_access_token)),
        )
        .service(
            web::resource("/account/{account_id}/transactions")
                .route(web::get().to(transactions::get_transactions_by_account)),
        )
        .service(
            web::resource("/account/{account_id}/transaction/{transaction_id}")
                .route(web::get().to(transactions::get_transaction_by_account)),
        )
        .service(
            web::resource("/account/{account_id}")
                .route(web::get().to(accounts::get_account))
                .route(web::post().to(accounts::post_account))
                .route(web::delete().to(accounts::delete_account)),
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
        );
}
