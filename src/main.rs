use aide::{axum::ApiRouter, openapi::OpenApi};
use axum::http::{header, Method};
use axum::{extract::DefaultBodyLimit, Extension};
use log::info;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};

use crate::database::{AppState, DatabaseConnection};
use crate::error::ServiceError;

mod api;
mod database;
mod docs;
pub mod env;
mod error;
mod models;
mod request_state;

mod import;

#[cfg(feature = "mail")]
mod mail;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    aide::gen::on_error(|error| {
        println!("{error}");
    });
    aide::gen::extract_schemas(true);

    let app_state = AppState::connect(env::DATABASE_URL.as_str()).await;

    let args: Vec<String> = std::env::args().collect();
    let import_sql_dump = args.iter().any(|a| a == "import-sql-dump");

    if import_sql_dump {
        let connection = app_state
            .pool
            .acquire()
            .await
            .map_err(|err| ServiceError::InternalServerError(err.to_string()))
            .unwrap();
        let mut db = DatabaseConnection { connection };

        let products_path =
            std::env::var("IMPORT_ASCII_PAY_PRODUCTS").unwrap_or_else(|_| "./".to_string());
        let sql_dump_path =
            std::env::var("IMPORT_ASCII_PAY_SQL_DUMP").unwrap_or_else(|_| "./".to_string());
        import::import(&mut db, &products_path, &sql_dump_path)
            .await
            .unwrap();

        return;
    }

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
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
                .allow_origin(Any),
        )
        .with_state(app_state);

    let addr = format!("{}:{}", env::API_HOST.as_str(), env::API_PORT.as_str());
    let addr = SocketAddr::from_str(&addr).unwrap();
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}
