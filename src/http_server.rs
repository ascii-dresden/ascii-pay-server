use std::time::Duration;

use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{middleware, App, HttpServer};
use log::info;

use crate::api as module_api;
use crate::api::graphql;
use crate::identity_service::IdentityService;
use crate::utils::{env, DatabasePool, RedisPool, ServiceResult};

/// Start a new actix server with the given database pool
async fn start_server(database_pool: DatabasePool, redis_pool: RedisPool) -> ServiceResult<()> {
    // Read config params from env
    let address = format!("{}:{}", env::HOST.as_str(), *env::HTTP_PORT);
    let schema = graphql::create_schema_with_context(database_pool.clone(), redis_pool.clone());

    info!("Start http server at {}", address);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_header()
            .allow_any_method()
            .allow_any_origin();

        App::new()
            // Move database pool
            .app_data(Data::new(database_pool.clone()))
            .app_data(Data::new(redis_pool.clone()))
            .app_data(Data::new(schema.clone()))
            .wrap(cors)
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
    })
    .keep_alive(Duration::from_secs(60))
    .bind(address)?
    .run()
    .await?;

    Ok(())
}

pub async fn start_http_server(
    database_pool: DatabasePool,
    redis_pool: RedisPool,
) -> ServiceResult<()> {
    start_server(database_pool, redis_pool).await
}