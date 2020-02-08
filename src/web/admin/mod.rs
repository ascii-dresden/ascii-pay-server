pub mod accounts;
pub mod categories;
pub mod dashboard;
pub mod products;
pub mod terminal;
pub mod transactions;

use actix_web::web;

/// Setup routes for admin ui
pub fn init(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/admin")
            .service(web::resource("").route(web::get().to(dashboard::get_dashboard)))
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
                web::resource("/account/invite/{account_id}")
                    .route(web::get().to(accounts::invite_get)),
            )
            .service(
                web::resource("/account/revoke/{account_id}")
                    .route(web::get().to(accounts::revoke_get)),
            )
            .service(
                web::resource("/account/remove-nfc/{account_id}")
                    .route(web::get().to(accounts::remove_nfc_get)),
            )
            .service(
                web::resource("/account/remove-barcode/{account_id}")
                    .route(web::get().to(accounts::remove_barcode_get)),
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
                web::resource("/product/remove-image/{product_id}")
                    .route(web::get().to(products::get_product_remove_image)),
            )
            .service(
                web::resource("/product/upload-image/{product_id}")
                    .route(web::post().to(products::post_product_upload_image)),
            )
            .service(
                web::resource("/product/{product_id}")
                    .route(web::post().to(products::post_product_edit))
                    .route(web::get().to(products::get_product_edit)),
            )
            // Setup categories mangement related routes
            .service(web::resource("/categories").route(web::get().to(categories::get_categories)))
            .service(
                web::resource("/category/create")
                    .route(web::post().to(categories::post_category_create))
                    .route(web::get().to(categories::get_category_create)),
            )
            .service(
                web::resource("/category/delete/{category_id}")
                    .route(web::get().to(categories::get_category_delete)),
            )
            .service(
                web::resource("/category/{category_id}")
                    .route(web::post().to(categories::post_category_edit))
                    .route(web::get().to(categories::get_category_edit)),
            )
            // Setup transaction mangement related routes
            .service(
                web::resource("/transactions/{account_id}")
                    .route(web::get().to(transactions::get_transactions)),
            )
            .service(
                web::resource("/transactions/generate/{account_id}")
                    .route(web::post().to(transactions::post_transaction_generate_random))
                    .route(web::get().to(transactions::get_transaction_generate_random)),
            )
            .service(
                web::resource("/transaction/execute/{account_id}")
                    .route(web::post().to(transactions::post_execute_transaction)),
            )
            .service(
                web::resource("/transaction/{account_id}/{transaction_id}")
                    .route(web::get().to(transactions::get_transaction_details)),
            )
            .service(web::resource("/terminal").route(web::get().to(terminal::get_terminal))),
    );
}
