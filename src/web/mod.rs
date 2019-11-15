mod accounts;
mod identity_policy;
mod index;
mod products;
mod transactions;
mod utils;

use actix_files as fs;
use actix_identity::IdentityService;
use actix_web::web;
use identity_policy::DbIdentityPolicy;

/// Setup routes for admin ui
pub fn init(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/")
            // Set identity service for encrypted cookies
            .wrap(IdentityService::new(DbIdentityPolicy::new()))
            // Setup static routes
            .service(fs::Files::new("/stylesheets", "static/stylesheets/"))
            .service(fs::Files::new("/images", "static/images/"))
            .service(fs::Files::new("/product/image", "img/"))
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
                web::resource("/product/remove-image/{product_id}")
                    .route(web::get().to(products::get_product_remove_image)),
            )
            .service(
                web::resource("/product/upload-image/{product_id}")
                    .route(web::post().to_async(products::post_product_upload_image)),
            )
            .service(
                web::resource("/product/{product_id}")
                    .route(web::post().to(products::post_product_edit))
                    .route(web::get().to(products::get_product_edit)),
            )
            // Setup transaction mangement related routes
            .service(
                web::resource("/transactions/{account_id}")
                    .route(web::get().to(transactions::get_transactions)),
            )
            .service(
                web::resource("/transaction/execute/{account_id}")
                    .route(web::post().to(transactions::post_execute_transaction)),
            ),
    );
}
