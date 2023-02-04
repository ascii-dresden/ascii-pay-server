use aide::OperationInput;
use aide::{axum::ApiRouter, openapi::OpenApi};
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::Method;
use axum::{async_trait, RequestPartsExt, TypedHeader};
use axum::{extract::DefaultBodyLimit, Extension};
use error::ServiceError;
use headers::authorization::Bearer;
use headers::Authorization;
use log::info;
use models::Session;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
};

use crate::database::{AppState, DatabaseConnection};

mod api;
mod database;
mod docs;
mod error;
mod models;

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
                .allow_methods([Method::GET, Method::POST])
                .allow_origin(Any),
        )
        .layer(CompressionLayer::new())
        .with_state(app_state);

    // run it with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// we can also write a custom extractor that grabs a connection from the pool
// which setup is appropriate depends on your application
pub struct RequestState {
    pub db: DatabaseConnection,
    pub session: Option<Session>,
}

#[async_trait]
impl<S> FromRequestParts<S> for RequestState
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ServiceError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);

        let connection = state
            .pool
            .acquire()
            .await
            .map_err(|err| ServiceError::InternalServerError(err.to_string()))?;
        let db = DatabaseConnection { connection };

        let session = if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            let session_token = bearer.token().to_owned();
            db.get_session_by_session_token(session_token).await?
        } else {
            None
        };

        Ok(Self { db, session })
    }
}

impl OperationInput for RequestState {}
