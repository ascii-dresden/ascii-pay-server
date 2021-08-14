pub mod graphql;
pub mod rest;
pub mod wallet;

use actix_web::web;

pub fn init(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/api/v1")
            .service(web::scope("/graphql").configure(graphql::init))
            .configure(rest::init),
    );
    wallet::init(config);
}
