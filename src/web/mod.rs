mod index;

use actix_files as fs;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::web;

lazy_static::lazy_static! {
pub  static ref SECRET_KEY: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));
}

pub fn init(config: &mut web::ServiceConfig) {
    let domain = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());

    config.service(
        web::scope("/")
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(SECRET_KEY.as_bytes())
                    .name("auth")
                    .path("/")
                    .domain(&domain)
                    .max_age_time(chrono::Duration::days(1))
                    .secure(false),
            ))
            .service(fs::Files::new("/stylesheets", "static/stylesheets/"))
            .service(web::resource("").route(web::get().to(index::index)))
            .service(web::resource("/login").route(web::post().to(index::login)))
            .service(web::resource("/logout").route(web::get().to(index::logout))),
    );
}
