use aes::Aes128;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use sublime_fuzzy as fuzzy;
use uuid::Uuid;

use super::ServiceResult;

lazy_static::lazy_static! {
    pub static ref COOKIE_KEY: [u8; 16] = hex!("000102030405060708090a0b0c0d0e0f");
    pub static ref COOKIE_IV: [u8; 16] = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
}

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
        .to_owned()
}

pub fn fuzzy_vec_match(search: &str, values: &[String]) -> Option<Vec<String>> {
    let join = values.join("");

    let result = match fuzzy::best_match(search, &join) {
        Some(result) => result,
        None => return None,
    };

    let mut start_index = 0;
    let vec: Vec<String> = values
        .iter()
        .map(|v| {
            let len = v.chars().count();
            let next_start_index = start_index + len;
            let matches = result
                .matches()
                .iter()
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

pub fn parse_obj_from_token<T>(token: &str) -> ServiceResult<T>
where
    T: DeserializeOwned,
{
    type Aes128Cbc = Cbc<Aes128, Pkcs7>;
    let cipher = Aes128Cbc::new_from_slices(COOKIE_KEY.as_ref(), COOKIE_IV.as_ref())?;

    let ciphertext = base64::decode(token)?;
    let buffer = cipher.decrypt_vec(&ciphertext)?;
    let obj: T = serde_json::from_slice(&buffer)?;

    Ok(obj)
}

pub fn create_token_from_obj<T>(obj: &T) -> ServiceResult<String>
where
    T: Serialize,
{
    type Aes128Cbc = Cbc<Aes128, Pkcs7>;
    let cipher = Aes128Cbc::new_from_slices(COOKIE_KEY.as_ref(), COOKIE_IV.as_ref())?;

    let buffer = serde_json::to_vec(obj)?;
    let ciphertext = cipher.encrypt_vec(&buffer);

    Ok(base64::encode(&ciphertext))
}
