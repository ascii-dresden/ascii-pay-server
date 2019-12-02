use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use uuid::Uuid;

/// Reference type for money values
pub type Money = i32;

/// Reference type to the current database implementation
pub type DB = diesel::pg::Pg;

/// Reference type to the current database connection
pub type DbConnection = PgConnection;

/// Reference type to the threaded pool of the current database connection
pub type Pool = r2d2::Pool<ConnectionManager<DbConnection>>;

/// Generate a new random uuid
pub fn generate_uuid() -> Uuid {
    Uuid::new_v4()
}

pub fn generate_uuid_str() -> String {
    generate_uuid()
        .to_hyphenated()
        .encode_upper(&mut Uuid::encode_buffer())
        .to_string()
}

pub trait Searchable {
    fn contains(&self, search: &str) -> bool;
}
