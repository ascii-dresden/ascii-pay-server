use actix_web::{middleware, web, App, HttpServer};
use handlebars::Handlebars;

use crate::api as module_api;
use crate::core::{Pool, ServiceError};
use crate::web as module_web;

pub fn start_server(pool: Pool) -> Result<(), ServiceError> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "".to_string())
        .parse::<i32>()
        .unwrap_or(8080);

    let address = format!("{}:{}", &host, port);

    let mut handlebars = Handlebars::new();
    handlebars
        .register_templates_directory(".html", "./static/templates")
        .unwrap();
    let handlebars_ref = web::Data::new(handlebars);

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .register_data(handlebars_ref.clone())
            .wrap(middleware::Logger::default())
            .configure(module_api::init)
            .configure(module_web::init)
    })
    .bind(address)?
    .run()
    .map_err(|_| ServiceError::InternalServerError)
}
