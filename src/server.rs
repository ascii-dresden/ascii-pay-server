use actix_web::{middleware, web, App, HttpServer};
use chrono::NaiveDateTime;
use handlebars::{Context, Handlebars, Helper, Output, RenderContext, RenderError};

use crate::api as module_api;
use crate::api::graphql;
use crate::core::{env, Pool, ServiceResult};
use crate::identity_service::IdentityService;
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

    let address = format!("{}:{}", env::HOST.as_str(), *env::PORT);

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

    let schema = graphql::create_schema_with_context(pool.clone());

    HttpServer::new(move || {
        App::new()
            // Move database pool
            .data(pool.clone())
            .data(schema.clone())
            // Set handlebars reference
            .app_data(handlebars_ref.clone())
            // Enable request/response compression support
            .wrap(middleware::Compress::default())
            // Enable logger
            .wrap(middleware::Logger::new(
                r#"%s: "%r" %b "%{Referer}i" "%{User-Agent}i" %T"#,
            ))
            // Set identity service for encrypted cookies
            .wrap(IdentityService::new())
            // Register api module
            .configure(module_api::init)
            // Register admin ui module
            .configure(module_web::init)
    })
    .keep_alive(60)
    .bind(address)?
    .run()
    .await?;

    Ok(())
}
