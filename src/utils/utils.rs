use std::convert::TryInto;

use aes::Aes128;
use bb8_redis::{redis, RedisConnectionManager};
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use diesel::prelude::*;
use rand::RngCore;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use sublime_fuzzy as fuzzy;
use uuid::Uuid;

use super::bb8_diesel::DieselConnectionManager;
use super::{ServiceError, ServiceResult};

lazy_static::lazy_static! {
    pub static ref COOKIE_KEY: [u8; 16] = hex!("000102030405060708090a0b0c0d0e0f");
    pub static ref COOKIE_IV: [u8; 16] = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
}

/// Reference type for money values
pub type Money = i32;

/// Reference type to the current database implementation
pub type DB = diesel::pg::Pg;

/// Reference type to the current database connection
pub type DatabaseConnection = PgConnection;
pub type RedisConnection = redis::aio::Connection;

/// Reference type to the threaded pool of the current database connection
pub type DatabasePool = bb8::Pool<DieselConnectionManager<DatabaseConnection>>;
pub type RedisPool = bb8::Pool<RedisConnectionManager>;

/// Generate a new random uuid
pub fn generate_uuid() -> Uuid {
    Uuid::new_v4()
}

pub fn generate_uuid_str() -> String {
    uuid_to_str(generate_uuid())
}

pub fn uuid_to_str(uuid: Uuid) -> String {
    uuid.hyphenated()
        .encode_upper(&mut Uuid::encode_buffer())
        .to_owned()
}

pub fn bytes_to_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|x| format!("{:02X}", x))
        .collect::<Vec<String>>()
        .join(" ")
}

pub fn str_to_bytes(s: &str) -> Vec<u8> {
    s.split(' ')
        .map(|x| u8::from_str_radix(x, 16).unwrap_or(0))
        .collect()
}

/// Generate a random byte sequence of length `length`.
pub fn generate_key_vec(length: usize) -> Vec<u8> {
    let mut data = vec![0u8; length];
    rand::thread_rng().fill_bytes(&mut data);
    data
}

pub fn generate_key_array<const N: usize>() -> [u8; N] {
    let mut data = [0u8; N];
    rand::thread_rng().fill_bytes(&mut data);
    data
}

pub fn vec_to_array<T, const N: usize>(v: Vec<T>) -> ServiceResult<[T; N]> {
    let len = v.len();
    let r: Result<[T; N], _> = v.try_into();
    r.map_err(|_| {
        ServiceError::InternalServerError(
            "Cannot convert Vec to Array!",
            format!("Expected a Vec of length {} but it was {}", N, len),
        )
    })
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
