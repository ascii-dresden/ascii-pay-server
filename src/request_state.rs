use aide::OperationInput;
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    RequestPartsExt, TypedHeader,
};
use headers::{authorization::Bearer, Authorization};

use crate::{
    database::{AppState, DatabaseConnection},
    error::ServiceError,
    models::Session,
};

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
