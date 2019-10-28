use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use uuid::Uuid;

pub type Money = i32;
pub type DbConnection = SqliteConnection;
pub type DB = diesel::sqlite::Sqlite;
pub type Pool = r2d2::Pool<ConnectionManager<DbConnection>>;

pub fn generate_uuid() -> String {
    Uuid::new_v4()
        .to_hyphenated()
        .encode_upper(&mut Uuid::encode_buffer())
        .to_string()
}
