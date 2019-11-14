use actix_web::{error::ResponseError, Error as ActixError, HttpResponse};
use derive_more::Display;
use diesel::result::DatabaseErrorKind;

/// Represent errors in the application
///
/// All `ServiceError`s can be transformed to http errors.
#[derive(Debug, Display)]
pub enum ServiceError {
    #[display(fmt = "Db Error: {}", _0)]
    DbError(String),

    #[display(fmt = "Bad Request: {}", _0)]
    BadRequest(String),

    #[display(fmt = "Internal Server Error")]
    InternalServerError,

    #[display(fmt = "Not Found")]
    NotFound,

    #[display(fmt = "Unauthorized")]
    Unauthorized,
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
        match error {
            diesel::result::Error::DatabaseError(kind, info) => {
                if let DatabaseErrorKind::UniqueViolation = kind {
                    let message = info.details().unwrap_or_else(|| info.message()).to_string();
                    return ServiceError::DbError(message);
                }
                ServiceError::InternalServerError
            }
            _ => ServiceError::InternalServerError,
        }
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(_: std::io::Error) -> ServiceError {
        ServiceError::InternalServerError
    }
}
impl From<handlebars::RenderError> for ServiceError {
    fn from(_: handlebars::RenderError) -> ServiceError {
        ServiceError::InternalServerError
    }
}
impl From<r2d2::Error> for ServiceError {
    fn from(_: r2d2::Error) -> ServiceError {
        ServiceError::InternalServerError
    }
}
impl From<uuid::parser::ParseError> for ServiceError {
    fn from(_: uuid::parser::ParseError) -> ServiceError {
        ServiceError::BadRequest("Invalid UUID".into())
    }
}
impl From<serde_json::Error> for ServiceError {
    fn from(_: serde_json::Error) -> ServiceError {
        ServiceError::InternalServerError
    }
}
/*
/// nightly - allow `?` on Option<T> to unwrap
impl From<std::option::NoneError> for ServiceError {
    fn from(_: std::option::NoneError) -> ServiceError {
        ServiceError::InternalServerError
    }
}
*/

/// Transform `ServiceError` to `HttpResponse`
impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::InternalServerError => {
                HttpResponse::InternalServerError().json("Internal Server Error, Please try later")
            }
            ServiceError::DbError(ref message) => HttpResponse::InternalServerError().json(message),
            ServiceError::BadRequest(ref message) => HttpResponse::BadRequest().json(message),
            ServiceError::NotFound => HttpResponse::NotFound().json("NotFound"),
            ServiceError::Unauthorized => HttpResponse::Unauthorized().json("Unauthorized"),
        }
    }
}
