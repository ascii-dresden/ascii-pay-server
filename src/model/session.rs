#![allow(clippy::from_over_into)]

use uuid::Uuid;

use crate::utils::{
    create_token_from_obj, generate_uuid, parse_obj_from_token, uuid_to_str, DatabasePool,
    RedisPool, ServiceResult,
};

use super::{redis, Account};

#[derive(Debug, Serialize, Deserialize)]
struct InnerSession {
    key: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, NewType)]
pub struct Session(String);

impl Session {
    pub fn new() -> ServiceResult<(Uuid, Self)> {
        let key = generate_uuid();
        let session = Session::from_key(key)?;
        Ok((key, session))
    }

    pub fn get_key(&self) -> ServiceResult<Uuid> {
        let obj = parse_obj_from_token::<InnerSession>(&self.0)?;
        Ok(obj.key)
    }

    pub fn from_key(key: Uuid) -> ServiceResult<Self> {
        let s = create_token_from_obj(&InnerSession { key })?;
        Ok(Session(s))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct LongtimeSession {
    account_id: Uuid,
}

pub async fn create_longtime_session(
    redis_pool: &RedisPool,
    account: &Account,
) -> ServiceResult<Session> {
    let (session_key, session) = Session::new()?;

    redis::create_data::<LongtimeSession>(
        redis_pool,
        &uuid_to_str(session_key),
        &LongtimeSession {
            account_id: account.id,
        },
        300,
    )
    .await?;

    Ok(session)
}

pub async fn get_longtime_session(
    database_pool: &DatabasePool,
    redis_pool: &RedisPool,
    session: &Session,
) -> ServiceResult<Account> {
    let session =
        redis::get_data::<LongtimeSession>(redis_pool, &uuid_to_str(session.get_key()?), 10 * 60)
            .await?;

    Account::get(database_pool, session.account_id).await
}

pub async fn delete_longtime_session(
    redis_pool: &RedisPool,
    session: &Session,
) -> ServiceResult<()> {
    redis::delete_data::<LongtimeSession>(redis_pool, &uuid_to_str(session.get_key()?)).await
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct OnetimeSession {
    account_id: Uuid,
}

pub async fn create_onetime_session(
    redis_pool: &RedisPool,
    account: &Account,
) -> ServiceResult<Session> {
    create_onetime_session_ttl(redis_pool, account, 10).await
}

pub async fn create_onetime_session_ttl(
    redis_pool: &RedisPool,
    account: &Account,
    ttl: i32,
) -> ServiceResult<Session> {
    let (session_key, session) = Session::new()?;

    redis::create_data::<OnetimeSession>(
        redis_pool,
        &uuid_to_str(session_key),
        &OnetimeSession {
            account_id: account.id,
        },
        ttl,
    )
    .await?;

    Ok(session)
}

pub async fn get_onetime_session(
    database_pool: &DatabasePool,
    redis_pool: &RedisPool,
    session: &Session,
) -> ServiceResult<Account> {
    let session =
        redis::get_delete_data::<OnetimeSession>(redis_pool, &uuid_to_str(session.get_key()?))
            .await?;

    Account::get(database_pool, session.account_id).await
}
