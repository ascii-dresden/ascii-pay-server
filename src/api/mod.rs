use actix_web::web;

lazy_static::lazy_static! {
pub  static ref SECRET_KEY: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));
}

pub fn init(config: &mut web::ServiceConfig) {
    let domain = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());
    // TODO: Implement API
    config.service(
        web::scope("/api/v1")
    );
}
