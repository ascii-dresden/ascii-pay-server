use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

/// Represent errors in the application
///
/// All `ServiceError`s can be transformed to http errors.
#[derive(Debug, Clone)]
pub enum ServiceError {
    InternalServerError(String),
    NotFound,
    NoneError,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ServiceError {}

/// Helper for `ServiceError` result
pub type ServiceResult<T> = Result<T, ServiceError>;

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ServiceError::InternalServerError(ref cause) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "cause": cause })),
            ),
            ServiceError::NotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "Not found",
                })),
            ),
            ServiceError::NoneError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "None type error",
                })),
            ),
        }
        .into_response()
    }
}
