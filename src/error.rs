use aide::OperationOutput;
use axum::{http::StatusCode, response::IntoResponse, Json};
use schemars::JsonSchema;
use serde_json::json;

/// Represent errors in the application
///
/// All `ServiceError`s can be transformed to http errors.
#[derive(Debug, Clone, JsonSchema, PartialEq)]
pub enum ServiceError {
    InternalServerError(String),
    NotFound,
    Unauthorized(&'static str),
    Forbidden,
    PaymentError(Vec<String>),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ServiceError {}

/// Helper for `ServiceError` result
pub type ServiceResult<T> = Result<T, ServiceError>;

impl From<sqlx::Error> for ServiceError {
    fn from(error: sqlx::Error) -> Self {
        ServiceError::InternalServerError(error.to_string())
    }
}

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

impl From<std::num::ParseIntError> for ServiceError {
    fn from(error: std::num::ParseIntError) -> Self {
        ServiceError::InternalServerError(error.to_string())
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(error: std::io::Error) -> Self {
        ServiceError::InternalServerError(error.to_string())
    }
}

impl From<openssl::error::ErrorStack> for ServiceError {
    fn from(error: openssl::error::ErrorStack) -> Self {
        ServiceError::InternalServerError(error.to_string())
    }
}

impl From<awc::error::SendRequestError> for ServiceError {
    fn from(error: awc::error::SendRequestError) -> Self {
        ServiceError::InternalServerError(error.to_string())
    }
}

#[cfg(feature = "mail")]
impl From<lettre::transport::smtp::Error> for ServiceError {
    fn from(error: lettre::transport::smtp::Error) -> Self {
        ServiceError::InternalServerError(format!("{}", error))
    }
}

#[cfg(feature = "mail")]
impl From<lettre::error::Error> for ServiceError {
    fn from(error: lettre::error::Error) -> Self {
        ServiceError::InternalServerError(format!("{}", error))
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
            ServiceError::PaymentError(cause) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "PaymentError",
                    "cause": cause
                })),
            ),
        }
        .into_response()
    }
}
