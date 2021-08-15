pub mod graphql;
pub mod rest;
pub mod wallet;
pub mod web;

use actix_web::web as a_web;

pub fn init(config: &mut a_web::ServiceConfig) {
    config.service(
        a_web::scope("/api/v1")
            .service(a_web::scope("/graphql").configure(graphql::init))
            .configure(rest::init),
    );
    wallet::init(config);
    web::init(config);
}
