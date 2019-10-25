use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DBError};
use uuid::Uuid;

pub type Money = i32;
pub type DbConnection = SqliteConnection;

#[derive(Debug)]
pub enum Error {
    DbError(String),
    InternalServerError,
    NotFound,
}
impl From<DBError> for Error {
    fn from(error: DBError) -> Error {
        match error {
            DBError::DatabaseError(kind, info) => {
                if let DatabaseErrorKind::UniqueViolation = kind {
                    let message = info.details().unwrap_or_else(|| info.message()).to_string();
                    return Error::DbError(message);
                }
                Error::InternalServerError
            }
            _ => Error::InternalServerError,
        }
    }
}

pub fn generate_uuid() -> String {
    Uuid::new_v4()
        .to_hyphenated()
        .encode_upper(&mut Uuid::encode_buffer())
        .to_string()
}
