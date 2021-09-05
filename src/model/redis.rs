use std::any::type_name;

use r2d2_redis::redis;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::utils::{RedisConnection, ServiceError, ServiceResult};

pub fn create_data_str(
    redis_conn: &mut RedisConnection,
    store: &str,
    key: &str,
    value: &str,
    ttl: i32,
) -> ServiceResult<()> {
    let result = redis::cmd("SET")
        .arg(format!("{}/{}", store, key))
        .arg(value)
        .arg("NX")
        .arg("EX")
        .arg(ttl)
        .query::<String>(redis_conn)?;

    if result != "OK" {
        return Err(ServiceError::NotFound);
    }

    Ok(())
}

pub fn get_data_str(
    redis_conn: &mut RedisConnection,
    storage: &str,
    key: &str,
    ttl: i32,
) -> ServiceResult<String> {
    let result = redis::cmd("GETEX")
        .arg(format!("{}/{}", storage, key))
        .arg("EX")
        .arg(ttl)
        .query::<String>(redis_conn)?;

    Ok(result)
}

pub fn get_delete_data_str(
    redis_conn: &mut RedisConnection,
    storage: &str,
    key: &str,
) -> ServiceResult<String> {
    let result = redis::cmd("GETDEL")
        .arg(format!("{}/{}", storage, key))
        .query::<String>(redis_conn)?;

    Ok(result)
}

pub fn delete_data_str(
    redis_conn: &mut RedisConnection,
    storage: &str,
    key: &str,
) -> ServiceResult<()> {
    let result = redis::cmd("DEL")
        .arg(format!("{}/{}", storage, key))
        .query::<String>(redis_conn)?;

    if result != "OK" {
        return Err(ServiceError::NotFound);
    }

    Ok(())
}

pub fn create_data<T>(
    redis_conn: &mut RedisConnection,
    key: &str,
    value: &T,
    ttl: i32,
) -> ServiceResult<()>
where
    T: Serialize,
{
    let data = serde_json::to_string(value)?;
    create_data_str(redis_conn, type_name::<T>(), key, &data, ttl)
}

pub fn get_data<T>(redis_conn: &mut RedisConnection, key: &str, ttl: i32) -> ServiceResult<T>
where
    T: DeserializeOwned,
{
    let data = get_data_str(redis_conn, type_name::<T>(), key, ttl)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn get_delete_data<T>(redis_conn: &mut RedisConnection, key: &str) -> ServiceResult<T>
where
    T: DeserializeOwned,
{
    let data = get_delete_data_str(redis_conn, type_name::<T>(), key)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn delete_data<T>(redis_conn: &mut RedisConnection, key: &str) -> ServiceResult<()>
where
    T: DeserializeOwned,
{
    delete_data_str(redis_conn, type_name::<T>(), key)
}
