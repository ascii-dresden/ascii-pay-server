pub mod overview;
pub mod settings;

use actix_web::web;

/// Setup routes for admin ui
pub fn init(config: &mut web::ServiceConfig) {
    config
            .service(web::resource("").route(web::get().to(overview::get_overview)))
            .service(
                web::resource("/transaction/{transaction_id}")
                    .route(web::get().to(overview::get_transaction_details)),
            )
            .service(
                web::resource("/settings")
                    .route(web::post().to(settings::post_settings))
                    .route(web::get().to(settings::get_settings)),
            )
            .service(
                web::resource("/settings/change-password")
                    .route(web::post().to(settings::post_change_password))
                    .route(web::get().to(settings::get_change_password)),
            )
            .service(
                web::resource("/settings/revoke-password")
                    .route(web::post().to(settings::post_revoke_password))
                    .route(web::get().to(settings::get_revoke_password)),
            )
            .service(
                web::resource("/settings/revoke-qr")
                    .route(web::post().to(settings::post_revoke_qr))
                    .route(web::get().to(settings::get_revoke_qr)),
            )
            .service(
                web::resource("/settings/revoke-nfc")
                    .route(web::post().to(settings::post_revoke_nfc))
                    .route(web::get().to(settings::get_revoke_nfc)),
            );
}
