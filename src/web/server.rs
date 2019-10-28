use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{middleware, web, App, HttpServer};

use crate::core::{ServiceError, Pool};
use crate::web::auth_handlers;

lazy_static::lazy_static! {
pub  static ref SECRET_KEY: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));
}

pub fn init(domain: &str, host: &str, port: i32, pool: Pool) -> Result<(), ServiceError> {
    let address = format!("{}:{}", &host, port);
    let domain = domain.to_string();

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(SECRET_KEY.as_bytes())
                    .name("auth")
                    .path("/")
                    .domain(&domain)
                    .max_age_time(chrono::Duration::days(1))
                    .secure(false),
            ))
            .data(web::JsonConfig::default().limit(4096))
            .service(
                web::scope("/api/v1")
                    .service(
                        web::resource("/auth")
                            .route(web::post().to_async(auth_handlers::login))
                            .route(web::delete().to(auth_handlers::logout))
                            .route(web::get().to(auth_handlers::get_me)),
                    ),
            )

    })
    .bind(address)?
    .run()
    .map_err(|_| ServiceError::InternalServerError)
}
