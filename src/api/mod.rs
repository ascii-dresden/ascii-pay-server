mod auth_handlers;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::web;

lazy_static::lazy_static! {
pub  static ref SECRET_KEY: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));
}

pub fn init(config: &mut web::ServiceConfig) {
    let domain = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());
    config.service(
        web::scope("/api/v1")
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(SECRET_KEY.as_bytes())
                    .name("auth")
                    .path("/api/v1")
                    .domain(&domain)
                    .max_age_time(chrono::Duration::days(1))
                    .secure(false),
            ))
            .data(web::JsonConfig::default().limit(4096))
            .service(
                web::resource("/auth")
                    .route(web::post().to_async(auth_handlers::login))
                    .route(web::delete().to(auth_handlers::logout))
                    .route(web::get().to(auth_handlers::get_me)),
            ),
    );
}
