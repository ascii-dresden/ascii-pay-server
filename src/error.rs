use aide::OperationOutput;
use axum::{http::StatusCode, response::IntoResponse, Json};
use schemars::JsonSchema;
use serde_json::json;

/// Represent errors in the application
///
/// All `ServiceError`s can be transformed to http errors.
#[derive(Debug, Clone, JsonSchema)]
pub enum ServiceError {
    InternalServerError(String),
    NotFound,
    Unauthorized(&'static str),
    Forbidden,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ServiceError {}

/// Helper for `ServiceError` result
pub type ServiceResult<T> = Result<T, ServiceError>;

impl From<block_modes::InvalidKeyIvLength> for ServiceError {
    fn from(error: block_modes::InvalidKeyIvLength) -> Self {
        ServiceError::InternalServerError(error.to_string())
    }
}

impl From<block_modes::BlockModeError> for ServiceError {
    fn from(error: block_modes::BlockModeError) -> Self {
        ServiceError::InternalServerError(error.to_string())
    }
}

impl OperationOutput for ServiceError {
    type Inner = String;
}
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
            ServiceError::Unauthorized(cause) => (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": cause,
                })),
            ),
            ServiceError::Forbidden => (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "Forbidden",
                })),
            ),
        }
        .into_response()
    }
}
