use actix_web::{middleware, App, HttpServer};

use crate::api as module_api;
use crate::api::graphql;
use crate::identity_service::IdentityService;
use crate::utils::{env, DatabasePool, RedisPool, ServiceResult};

/// Start a new actix server with the given database pool
async fn start_server(database_pool: DatabasePool, redis_pool: RedisPool) -> ServiceResult<()> {
    // Read config params from env

    let address = format!("{}:{}", env::HOST.as_str(), *env::PORT);
    let schema = graphql::create_schema_with_context(database_pool.clone(), redis_pool.clone());

    println!("Start http server at {}", address);

    HttpServer::new(move || {
        App::new()
            // Move database pool
            .data(database_pool.clone())
            .data(redis_pool.clone())
            .data(schema.clone())
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
    .keep_alive(60)
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
