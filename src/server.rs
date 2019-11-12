use actix_web::{middleware, web, App, HttpServer};
use handlebars::{Handlebars, RenderError, RenderContext, Helper, Context, Output};

use crate::api as module_api;
use crate::core::{Pool, ServiceError};
use crate::web as module_web;

fn currency_helper(
    helper: &Helper, 
    _: &Handlebars, 
    _: &Context, 
    _: &mut RenderContext, 
    out: &mut dyn Output
) -> Result<(), RenderError> {
    let param = helper.param(0).unwrap();
    if let Some(cents) = param.value().as_f64() {
        out.write(&format!("{:.2}", cents / 100.0))?;
    } 
    Ok(())
}

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
    handlebars.register_helper("currency", Box::new(currency_helper));
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
