use actix_web::{error::ResponseError, Error as ActixError, HttpResponse};
use derive_more::Display;
use log::{error, warn};

/// Represent errors in the application
///
/// All `ServiceError`s can be transformed to http errors.
#[derive(Debug, Display, Clone)]
pub enum ServiceError {
    #[display(fmt = "Bad Request: '{}'\n{}", _0, _1)]
    BadRequest(&'static str, String),

    #[display(fmt = "Internal Server Error: '{}'\n{}", _0, _1)]
    InternalServerError(&'static str, String),

    #[display(fmt = "Transaction canceled: {}", _0)]
    TransactionCancelled(String),

    #[display(fmt = "Transaction error: {}", _0)]
    TransactionError(String),

    #[display(fmt = "Not Found")]
    NotFound,

    #[display(fmt = "Unauthorized: '{}'", _0)]
    Unauthorized(&'static str),

    #[display(fmt = "You have insufficient privileges to view this site")]
    InsufficientPrivileges,

    #[display(fmt = "Error sending mail: {}", _0)]
    MailError(String),

    #[display(fmt = "Cannot access none reference")]
    NoneError,
}

impl ServiceError {
    pub fn actix(self) -> ActixError {
        self.into()
    }
}

/// Helper for `ServiceError` result
pub type ServiceResult<T> = Result<T, ServiceError>;

impl From<diesel::result::Error> for ServiceError {
    fn from(error: diesel::result::Error) -> Self {
        if diesel::result::Error::NotFound == error {
            return ServiceError::NotFound;
        }
        warn!("Database error: {}", error);
        ServiceError::InternalServerError("Database error", format!("{}", error))
    }
}

// impl From<diesel_migrations::RunMigrationsError> for ServiceError {
//     fn from(error: diesel_migrations::RunMigrationsError) -> Self {
//         ServiceError::InternalServerError("Database error", format!("{}", error))
//     }
// }

impl From<bb8_redis::redis::RedisError> for ServiceError {
    fn from(error: bb8_redis::redis::RedisError) -> Self {
        ServiceError::InternalServerError("Redis error", format!("{}", error))
    }
}

impl From<diesel::r2d2::Error> for ServiceError {
    fn from(error: diesel::r2d2::Error) -> Self {
        ServiceError::InternalServerError("Diesel r2d2 error", format!("{}", error))
    }
}

impl From<bb8::RunError<diesel::r2d2::Error>> for ServiceError {
    fn from(error: bb8::RunError<diesel::r2d2::Error>) -> Self {
        ServiceError::InternalServerError("Diesel r2d2 error", format!("{}", error))
    }
}

impl From<bb8::RunError<bb8_redis::redis::RedisError>> for ServiceError {
    fn from(error: bb8::RunError<bb8_redis::redis::RedisError>) -> Self {
        ServiceError::InternalServerError("Redis bb8 error", format!("{}", error))
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(error: std::io::Error) -> Self {
        ServiceError::InternalServerError("IO error", format!("{}", error))
    }
}

impl From<uuid::Error> for ServiceError {
    fn from(error: uuid::Error) -> Self {
        ServiceError::BadRequest("Invalid UUID", format!("{}", error))
    }
}

impl From<serde_json::Error> for ServiceError {
    fn from(error: serde_json::Error) -> Self {
        ServiceError::InternalServerError("Serialization error", format!("{}", error))
    }
}

impl From<actix_http::Error> for ServiceError {
    fn from(error: actix_http::Error) -> Self {
        ServiceError::InternalServerError("Http error", format!("{}", error))
    }
}

impl From<awc::error::SendRequestError> for ServiceError {
    fn from(error: awc::error::SendRequestError) -> Self {
        ServiceError::InternalServerError("actix client error", format!("{}", error))
    }
}

impl From<base64::DecodeError> for ServiceError {
    fn from(error: base64::DecodeError) -> Self {
        ServiceError::InternalServerError("Base64 error", format!("{}", error))
    }
}

impl From<std::string::FromUtf8Error> for ServiceError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        ServiceError::InternalServerError("Utf8Encoding error", format!("{}", error))
    }
}

impl From<block_modes::InvalidKeyIvLength> for ServiceError {
    fn from(error: block_modes::InvalidKeyIvLength) -> Self {
        ServiceError::InternalServerError("Encryption error", format!("{}", error))
    }
}

impl From<block_modes::BlockModeError> for ServiceError {
    fn from(error: block_modes::BlockModeError) -> Self {
        ServiceError::InternalServerError("Encryption error", format!("{}", error))
    }
}

// impl From<lettre::smtp::error::Error> for ServiceError {
//     fn from(error: lettre::smtp::error::Error) -> Self {
//         ServiceError::MailError(error.to_string())
//     }
// }

impl From<actix_web::http::header::ToStrError> for ServiceError {
    fn from(error: actix_web::http::header::ToStrError) -> Self {
        ServiceError::BadRequest(
            "Request contained invalid CRON_SECRET header value",
            format!("{}", error),
        )
    }
}

impl From<lettre_email::error::Error> for ServiceError {
    fn from(error: lettre_email::error::Error) -> Self {
        ServiceError::InternalServerError("Mail construction error", format!("{}", error))
    }
}

impl From<std::num::ParseIntError> for ServiceError {
    fn from(error: std::num::ParseIntError) -> Self {
        ServiceError::BadRequest("Illegal number string", format!("{}", error))
    }
}

impl From<actix_http::error::PayloadError> for ServiceError {
    fn from(error: actix_http::error::PayloadError) -> Self {
        ServiceError::InternalServerError("Http client payload error", format!("{}", error))
    }
}

impl From<openssl::error::ErrorStack> for ServiceError {
    fn from(error: openssl::error::ErrorStack) -> Self {
        ServiceError::InternalServerError("OpenSSL error", format!("{}", error))
    }
}

impl From<actix_rt::task::JoinError> for ServiceError {
    fn from(error: actix_rt::task::JoinError) -> Self {
        ServiceError::InternalServerError("Actix join error", format!("{}", error))
    }
}

impl From<std::str::Utf8Error> for ServiceError {
    fn from(error: std::str::Utf8Error) -> Self {
        ServiceError::InternalServerError("Utf8 conversion error", format!("{}", error))
    }
}

impl From<async_graphql::Error> for ServiceError {
    fn from(error: async_graphql::Error) -> Self {
        ServiceError::InternalServerError("GraphQL error", format!("{:?}", error))
    }
}

impl From<http::Error> for ServiceError {
    fn from(error: http::Error) -> Self {
        ServiceError::InternalServerError("Http error", format!("{:?}", error))
    }
}

impl From<argon2rs::verifier::DecodeError> for ServiceError {
    fn from(error: argon2rs::verifier::DecodeError) -> Self {
        ServiceError::InternalServerError("Hash error", format!("{:?}", error))
    }
}

impl<T> From<std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, T>>> for ServiceError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, T>>) -> Self {
        ServiceError::InternalServerError("Lock poison error", format!("{:?}", error))
    }
}

impl<T> From<std::sync::PoisonError<std::sync::RwLockReadGuard<'_, T>>> for ServiceError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockReadGuard<'_, T>>) -> Self {
        ServiceError::InternalServerError("Lock poison error", format!("{:?}", error))
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for ServiceError
where
    T: std::fmt::Debug,
{
    fn from(error: tokio::sync::mpsc::error::SendError<T>) -> Self {
        ServiceError::InternalServerError("Tokio mpsc send error", format!("{:?}", error))
    }
}

impl From<ServiceError> for grpcio::Error {
    fn from(_error: ServiceError) -> Self {
        grpcio::Error::RemoteStopped
    }
}

/// Transform `ServiceError` to `HttpResponse`
impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::InternalServerError(ref source, ref cause) => {
                HttpResponse::InternalServerError().json(json!({
                    "message": "Internal Server Error, Please try again later",
                    "source": source,
                    "cause": cause
                }))
            }
            ServiceError::TransactionCancelled(ref message) => {
                HttpResponse::Conflict().json(json!({
                    "message": "Payment canceled",
                    "cause": message
                }))
            }
            ServiceError::TransactionError(ref message) => HttpResponse::Conflict().json(json!({
                "message": "Payment error",
                "cause": message
            })),
            ServiceError::BadRequest(ref source, ref cause) => {
                HttpResponse::BadRequest().json(json!({
                    "message": "Internal Server Error, Please try again later",
                    "source": source,
                    "cause": cause
                }))
            }
            ServiceError::NotFound => HttpResponse::NotFound().json(json!({
                "message": "Not found"
            })),
            ServiceError::NoneError => HttpResponse::InternalServerError().json(json!({
                "message": "None type error"
            })),
            ServiceError::Unauthorized(_) => HttpResponse::Unauthorized().json(json!({
                "message": "Unauthorized"
            })),
            ServiceError::InsufficientPrivileges => HttpResponse::Forbidden().json(json!({
                "message": "Forbidden"
            })),
            ServiceError::MailError(ref mail_err) => {
                HttpResponse::InternalServerError().json(json!({
                    "message": "An error occured when trying to send an email.",
                    "cause": mail_err.to_string()
                }))
            }
        }
    }
}

pub fn log_result<R, E>(result: Result<R, E>) -> Result<R, E>
where
    E: std::fmt::Debug,
{
    if let Err(e) = &result {
        error!("{:?}", e);
    }
    result
}
