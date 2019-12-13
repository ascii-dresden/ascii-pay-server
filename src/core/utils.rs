use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use uuid::Uuid;
use sublime_fuzzy as fuzzy;

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

pub fn fuzzy_vec_match(search: &str, values: &[String]) -> Option<Vec<String>> {
    let join = values.join("");

    let result = match fuzzy::best_match(search, &join) {
        Some(result) => result,
        None => return None,
    };

    let mut start_index = 0;
    let vec: Vec<String> = values.iter()
        .map(|v| {
            let len = v.chars().count();
            let next_start_index = start_index + len;
            let matches = result.matches().iter()
                .filter(|i| start_index <= **i && **i < next_start_index)
                .map(|i| *i - start_index)
                .collect();
            let m = fuzzy::Match::with(result.score(), matches);
            start_index = next_start_index;

            fuzzy::format_simple(&m, v, "<b>", "</b>")
        })
        .collect();

    Some(vec)
}
