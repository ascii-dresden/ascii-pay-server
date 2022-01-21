mod wallet_routes;

use actix_web::web;

pub fn init(config: &mut web::ServiceConfig) {
    config
        .service(
            web::resource("/v1/devices/{device_id}/registrations/{pass_type_id}/{serial_number}")
                .route(web::post().to(wallet_routes::register_device))
                .route(web::delete().to(wallet_routes::unregister_device)),
        )
        .service(
            web::resource("/v1/devices/{device_id}/registrations/{pass_type_id}")
                .route(web::get().to(wallet_routes::update_passes)),
        )
        .service(
            web::resource("/v1/passes/{pass_type_id}/{serial_number}")
                .route(web::get().to(wallet_routes::pass_delivery)),
        )
        .service(web::resource("/v1/log").route(web::post().to(wallet_routes::log)))
        .service(
            web::resource("/v1/AsciiPayCard")
                .route(web::get().to(wallet_routes::forward_pass)),
        )
        .service(
            web::resource("/v1/AsciiPayCard.pkpass")
                .route(web::get().to(wallet_routes::create_pass)),
        );
}
