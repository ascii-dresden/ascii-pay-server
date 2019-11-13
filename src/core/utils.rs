use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use uuid::Uuid;

/// Reference type for money values
pub type Money = i32;

/// Reference type to the current database implementation
pub type DB = diesel::sqlite::Sqlite;

/// Reference type to the current database connection
pub type DbConnection = SqliteConnection;

/// Reference type to the threaded pool of the current database connection
pub type Pool = r2d2::Pool<ConnectionManager<DbConnection>>;

/// Generate a new random uuid
pub fn generate_uuid() -> String {
    Uuid::new_v4()
        .to_hyphenated()
        .encode_upper(&mut Uuid::encode_buffer())
        .to_string()
}
