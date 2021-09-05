use actix_web::http::header::ToStrError;
use actix_web::{error::ResponseError, Error as ActixError, HttpResponse};
use derive_more::Display;
use lettre::smtp::error::Error as LettreError;

/// Represent errors in the application
///
/// All `ServiceError`s can be transformed to http errors.
#[derive(Debug, Display, Clone)]
pub enum ServiceError {
    #[display(fmt = "Bad Request: '{}'\n{}", _0, _1)]
    BadRequest(&'static str, String),

    #[display(fmt = "Internal Server Error: '{}'\n{}", _0, _1)]
    InternalServerError(&'static str, String),

    #[display(fmt = "Not Found")]
    NotFound,

    #[display(fmt = "Unauthorized")]
    Unauthorized,

    #[display(fmt = "You have insufficient privileges to view this site")]
    InsufficientPrivileges,

    #[display(fmt = "Error sending mail: {}", _0)]
    MailError(String),

    #[display(fmt = "Cannot access none reference")]
    NoneError,

    #[display(fmt = "Request should be redirected to: {}". _0)]
    Redirect(String),
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
        println!("Database error: {}", error);
        ServiceError::InternalServerError("Database error", format!("{}", error))
    }
}

impl From<diesel_migrations::RunMigrationsError> for ServiceError {
    fn from(error: diesel_migrations::RunMigrationsError) -> Self {
        ServiceError::InternalServerError("Database error", format!("{}", error))
    }
}

impl From<r2d2_redis::redis::RedisError> for ServiceError {
    fn from(error: r2d2_redis::redis::RedisError) -> Self {
        println!("Redis error: {}", error);
        ServiceError::InternalServerError("Redis error", format!("{}", error))
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(error: std::io::Error) -> Self {
        ServiceError::InternalServerError("IO error", format!("{}", error))
    }
}

impl From<r2d2::Error> for ServiceError {
    fn from(error: r2d2::Error) -> Self {
        ServiceError::InternalServerError("r2d2 error", format!("{}", error))
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

impl From<actix_multipart::MultipartError> for ServiceError {
    fn from(error: actix_multipart::MultipartError) -> Self {
        ServiceError::InternalServerError("Error in Multipart stream", format!("{}", error))
    }
}

impl From<actix_http::Error> for ServiceError {
    fn from(error: actix_http::Error) -> Self {
        ServiceError::InternalServerError("Http error", format!("{}", error))
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

impl From<LettreError> for ServiceError {
    fn from(error: LettreError) -> Self {
        ServiceError::MailError(error.to_string())
    }
}

impl From<ToStrError> for ServiceError {
    fn from(error: ToStrError) -> Self {
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

impl From<actix_http::client::SendRequestError> for ServiceError {
    fn from(error: actix_http::client::SendRequestError) -> Self {
        ServiceError::InternalServerError("Http client (actix) error", format!("{}", error))
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

impl From<ServiceError> for grpc::Error {
    fn from(error: ServiceError) -> Self {
        grpc::Error::Panic(format!("{:?}", error))
    }
}

/*
/// nightly - allow `?` on Option<T> to unwrap
impl From<std::option::NoneError> for ServiceError {
    fn from(error: std::option::NoneError) -> ServiceError {
        ServiceError::InternalServerError("None error", format!("{}", error))
    }
}
*/

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
            ServiceError::Unauthorized => HttpResponse::Unauthorized().json(json!({
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
            ServiceError::Redirect(ref url) => HttpResponse::Found()
                .set_header(actix_web::http::header::LOCATION, url.as_str())
                .finish(),
        }
    }
}
