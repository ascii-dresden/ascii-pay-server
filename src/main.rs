use aide::{axum::ApiRouter, openapi::OpenApi};
use axum::http::Method;
use axum::{extract::DefaultBodyLimit, Extension};
use log::info;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
};

use crate::database::AppState;

mod api;
mod database;
mod docs;
mod error;
mod models;
mod request_state;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    aide::gen::on_error(|error| {
        println!("{error}");
    });
    aide::gen::extract_schemas(true);

    let db_connection_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ascii:ascii@localhost:5432/ascii-pay".to_string());

    let api_host = std::env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    let api_port = std::env::var("API_PORT").unwrap_or_else(|_| "3000".to_string());

    let app_state = AppState::connect(&db_connection_str).await;

    let mut api = OpenApi::default();

    let app = ApiRouter::new()
        .nest_api_service("/api/v1", api::init(app_state.clone()))
        .nest_api_service("/docs", docs::docs_routes())
        .finish_api_with(&mut api, docs::api_docs)
        .layer(Extension(Arc::new(api)))
        .layer(DefaultBodyLimit::disable())
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_origin(Any),
        )
        .layer(CompressionLayer::new())
        .with_state(app_state);

    // run it with hyper

    let addr = format!("{api_host}:{api_port}");
    let addr = SocketAddr::from_str(&addr).unwrap();
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
