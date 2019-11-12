use actix_web::{error::ResponseError, HttpResponse};
use derive_more::Display;
use diesel::result::{DatabaseErrorKind, Error as DBError};

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

impl From<DBError> for ServiceError {
    fn from(error: DBError) -> ServiceError {
        match error {
            DBError::DatabaseError(kind, info) => {
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
