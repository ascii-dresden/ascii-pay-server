use actix_web::{error::ResponseError, Error as ActixError, HttpResponse};
use derive_more::Display;

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

    #[display(fmt = "Session is expired")]
    Expired,
}

impl ServiceError {
    pub fn actix(self) -> ActixError {
        self.into()
    }
}

/// Helper for `ServiceError` result
pub type ServiceResult<T> = Result<T, ServiceError>;

impl From<diesel::result::Error> for ServiceError {
    fn from(error: diesel::result::Error) -> ServiceError {
        ServiceError::InternalServerError("Database error", format!("{}", error))
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(error: std::io::Error) -> ServiceError {
        ServiceError::InternalServerError("IO error", format!("{}", error))
    }
}
impl From<handlebars::RenderError> for ServiceError {
    fn from(error: handlebars::RenderError) -> ServiceError {
        ServiceError::InternalServerError("Render error", format!("{}", error))
    }
}
impl From<r2d2::Error> for ServiceError {
    fn from(error: r2d2::Error) -> ServiceError {
        ServiceError::InternalServerError("r2d2 error", format!("{}", error))
    }
}
impl From<uuid::parser::ParseError> for ServiceError {
    fn from(error: uuid::parser::ParseError) -> ServiceError {
        ServiceError::BadRequest("Invalid UUID", format!("{}", error))
    }
}
impl From<serde_json::Error> for ServiceError {
    fn from(error: serde_json::Error) -> ServiceError {
        ServiceError::InternalServerError("Serialization error", format!("{}", error))
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
                    "message": "Internal Server Error, Please try later",
                    "source": source,
                    "cause": cause
                }))
            }
            ServiceError::BadRequest(ref source, ref cause) => {
                HttpResponse::BadRequest().json(json!({
                    "message": "Internal Server Error, Please try later",
                    "source": source,
                    "cause": cause
                }))
            }
            ServiceError::NotFound => HttpResponse::NotFound().json("NotFound"),
            ServiceError::Unauthorized => HttpResponse::Unauthorized().json("Unauthorized"),
            ServiceError::Expired => {
                HttpResponse::Unauthorized().json("Session has expired, login again")
            }
        }
    }
}
