use actix_web::{error::ResponseError, Error as ActixError, HttpResponse};
use derive_more::Display;
use lettre::smtp::error::Error as LettreError;

pub const AUTH_COOKIE_NAME: &str = "auth";

/// Represent errors in the application
///
/// All `ServiceError`s can be transformed to http errors.
#[derive(Debug, Display)]
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
    MailError(LettreError),
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
        ServiceError::InternalServerError("Database error", format!("{}", error))
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(error: std::io::Error) -> Self {
        ServiceError::InternalServerError("IO error", format!("{}", error))
    }
}

impl From<handlebars::RenderError> for ServiceError {
    fn from(error: handlebars::RenderError) -> Self {
        ServiceError::InternalServerError("Render error", format!("{}", error))
    }
}

impl From<r2d2::Error> for ServiceError {
    fn from(error: r2d2::Error) -> Self {
        ServiceError::InternalServerError("r2d2 error", format!("{}", error))
    }
}

impl From<uuid::parser::ParseError> for ServiceError {
    fn from(error: uuid::parser::ParseError) -> Self {
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
        ServiceError::MailError(error)
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
            ServiceError::NotFound => HttpResponse::NotFound().json("NotFound"),
            ServiceError::Unauthorized => HttpResponse::Unauthorized().json("Unauthorized"),
            ServiceError::InsufficientPrivileges => {
                HttpResponse::Unauthorized().json("Insufficient Privileges")
            }
            ServiceError::MailError(ref mail_err) => {
                HttpResponse::InternalServerError().json(json!({
                    "message": "An error occured when trying to send an email.",
                    "cause": format!("{}", mail_err)
                }))
            }
        }
    }
}
