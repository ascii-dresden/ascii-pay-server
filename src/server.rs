use actix_identity::IdentityService;
use actix_web::{middleware, web, App, HttpServer};
use chrono::NaiveDateTime;
use handlebars::{Context, Handlebars, Helper, Output, RenderContext, RenderError};

use crate::api as module_api;
use crate::core::{Pool, ServiceResult};
use crate::identity_policy::DbIdentityPolicy;
use crate::web as module_web;

/// Helper function for handlebars. Converts cents to euros
fn currency_helper(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    if let Some(param) = helper.param(0) {
        if let Some(cents) = param.value().as_f64() {
            out.write(&format!("{:.2}", cents / 100.0))?;
        }
    }
    Ok(())
}

fn format_datetime_helper(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    if let Some(param) = helper.param(0) {
        if let Some(datetime) = param.value().as_str() {
            match NaiveDateTime::parse_from_str(datetime, "%Y-%m-%dT%H:%M:%S%.f") {
                Ok(d) => out.write(&d.format("%d.%m.%Y - %H:%M").to_string())?,
                Err(_) => out.write(datetime)?,
            };
        }
    }
    Ok(())
}

/// Start a new actix server with the given database pool
pub async fn start_server(pool: Pool) -> ServiceResult<()> {
    // Read config params from env
    let host = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let address = format!("{}:{}", &host, port);

    let mut handlebars = Handlebars::new();

    // Set handlebars template directory
    handlebars
        .register_templates_directory(".handlebars", "./static/templates")
        .unwrap();

    // Set handlebars helper function for cent/euro converter
    handlebars.register_helper("currency", Box::new(currency_helper));
    handlebars.register_helper("format_datetime", Box::new(format_datetime_helper));

    // Move handlebars reference to actix
    let handlebars_ref = web::Data::new(handlebars);

    HttpServer::new(move || {
        App::new()
            // Move database pool
            .data(pool.clone())
            // Set handlebars reference
            .app_data(handlebars_ref.clone())
            // Logger
            .wrap(middleware::Logger::default())
            // Set identity service for encrypted cookies
            .wrap(IdentityService::new(DbIdentityPolicy::new()))
            // Register api module
            .configure(module_api::init)
            // Register admin ui module
            .configure(module_web::init)
    })
    .bind(address)?
    .run()
    .await?;

    Ok(())
}
