use diesel::result::{DatabaseErrorKind, Error as DBError};
use std::io::Error as IOError;
use uuid::parser::ParseError;
use derive_more::Display;
use actix_web::{error::ResponseError, HttpResponse};


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

impl From<IOError> for ServiceError {
    fn from(_: IOError) -> ServiceError {
        ServiceError::InternalServerError
    }
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::InternalServerError => HttpResponse::InternalServerError().json("Internal Server Error, Please try later"),
            ServiceError::DbError(ref message) => HttpResponse::InternalServerError().json(message),
            ServiceError::BadRequest(ref message) => HttpResponse::BadRequest().json(message),
            ServiceError::NotFound => HttpResponse::NotFound().json("NotFound"),
            ServiceError::Unauthorized => HttpResponse::Unauthorized().json("Unauthorized"),
        }
    }
}

impl From<ParseError> for ServiceError {
    fn from(_: ParseError) -> ServiceError {
        ServiceError::BadRequest("Invalid UUID".into())
    }
}
