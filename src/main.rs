use axum::extract::DefaultBodyLimit;
use axum::http::Method;
use axum::Router;
use log::info;
use std::net::SocketAddr;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
};

use crate::database::Database;

mod api;
mod database;
mod error;
mod models;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let db_connection_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ascii:ascii@localhost:5432/ascii-pay".to_string());

    let database = Database::connect(&db_connection_str).await;

    // build our application with some routes
    let app = Router::new()
        .nest("/api/v1", api::init())
        .layer(DefaultBodyLimit::disable())
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_origin(Any),
        )
        .layer(CompressionLayer::new())
        .with_state(database);

    // run it with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
