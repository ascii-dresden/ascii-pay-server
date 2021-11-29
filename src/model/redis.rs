use std::{any::type_name, ops::DerefMut};

use bb8_redis::redis;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::utils::{RedisPool, ServiceError, ServiceResult};

pub async fn create_data_str(
    redis_pool: &RedisPool,
    store: &str,
    key: &str,
    value: &str,
    ttl: i32,
) -> ServiceResult<()> {
    let result: bool = redis::cmd("SET")
        .arg(format!("{}/{}", store, key))
        .arg(value)
        .arg("NX")
        .arg("EX")
        .arg(ttl)
        .query_async(redis_pool.get().await?.deref_mut())
        .await?;

    if !result {
        return Err(ServiceError::NotFound);
    }

    Ok(())
}

pub async fn get_data_str(
    redis_pool: &RedisPool,
    storage: &str,
    key: &str,
    ttl: i32,
) -> ServiceResult<String> {
    let result: String = redis::cmd("GETEX")
        .arg(format!("{}/{}", storage, key))
        .arg("EX")
        .arg(ttl)
        .query_async(redis_pool.get().await?.deref_mut())
        .await?;

    Ok(result)
}

pub async fn get_delete_data_str(
    redis_pool: &RedisPool,
    storage: &str,
    key: &str,
) -> ServiceResult<String> {
    let result: String = redis::cmd("GETDEL")
        .arg(format!("{}/{}", storage, key))
        .query_async(redis_pool.get().await?.deref_mut())
        .await?;

    Ok(result)
}

pub async fn delete_data_str(
    redis_pool: &RedisPool,
    storage: &str,
    key: &str,
) -> ServiceResult<()> {
    let result: bool = redis::cmd("DEL")
        .arg(format!("{}/{}", storage, key))
        .query_async(redis_pool.get().await?.deref_mut())
        .await?;

    if !result {
        return Err(ServiceError::NotFound);
    }

    Ok(())
}

pub async fn create_data<T>(
    redis_pool: &RedisPool,
    key: &str,
    value: &T,
    ttl: i32,
) -> ServiceResult<()>
where
    T: Serialize,
{
    let data = serde_json::to_string(value)?;
    create_data_str(redis_pool, type_name::<T>(), key, &data, ttl).await
}

pub async fn get_data<T>(redis_pool: &RedisPool, key: &str, ttl: i32) -> ServiceResult<T>
where
    T: DeserializeOwned,
{
    let data = get_data_str(redis_pool, type_name::<T>(), key, ttl).await?;
    Ok(serde_json::from_str(&data)?)
}

pub async fn get_delete_data<T>(redis_pool: &RedisPool, key: &str) -> ServiceResult<T>
where
    T: DeserializeOwned,
{
    let data = get_delete_data_str(redis_pool, type_name::<T>(), key).await?;
    Ok(serde_json::from_str(&data)?)
}

pub async fn delete_data<T>(redis_pool: &RedisPool, key: &str) -> ServiceResult<()>
where
    T: DeserializeOwned,
{
    delete_data_str(redis_pool, type_name::<T>(), key).await
}
