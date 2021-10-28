use uuid::Uuid;

use crate::utils::{
    create_token_from_obj, generate_uuid, parse_obj_from_token, uuid_to_str, DatabaseConnection,
    RedisConnection, ServiceResult,
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

pub fn create_longtime_session(
    redis_conn: &mut RedisConnection,
    account: &Account,
) -> ServiceResult<Session> {
    let (session_key, session) = Session::new()?;

    redis::create_data::<LongtimeSession>(
        redis_conn,
        &uuid_to_str(session_key),
        &LongtimeSession {
            account_id: account.id,
        },
        300,
    )?;

    Ok(session)
}

pub fn get_longtime_session(
    database_conn: &DatabaseConnection,
    redis_conn: &mut RedisConnection,
    session: &Session,
) -> ServiceResult<Account> {
    let session =
        redis::get_data::<LongtimeSession>(redis_conn, &uuid_to_str(session.get_key()?), 10 * 60)?;

    let account = Account::get(database_conn, session.account_id)?;

    Ok(account)
}

pub fn delete_longtime_session(
    redis_conn: &mut RedisConnection,
    session: &Session,
) -> ServiceResult<()> {
    redis::delete_data::<LongtimeSession>(redis_conn, &uuid_to_str(session.get_key()?))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct OnetimeSession {
    account_id: Uuid,
}

pub fn create_onetime_session(
    redis_conn: &mut RedisConnection,
    account: &Account,
) -> ServiceResult<Session> {
    create_onetime_session_ttl(redis_conn, account, 10)
}

pub fn create_onetime_session_ttl(
    redis_conn: &mut RedisConnection,
    account: &Account,
    ttl: i32,
) -> ServiceResult<Session> {
    let (session_key, session) = Session::new()?;

    redis::create_data::<OnetimeSession>(
        redis_conn,
        &uuid_to_str(session_key),
        &OnetimeSession {
            account_id: account.id,
        },
        ttl,
    )?;

    Ok(session)
}

pub fn get_onetime_session(
    database_conn: &DatabaseConnection,
    redis_conn: &mut RedisConnection,
    session: &Session,
) -> ServiceResult<Account> {
    let session =
        redis::get_delete_data::<OnetimeSession>(redis_conn, &uuid_to_str(session.get_key()?))?;

    let account = Account::get(database_conn, session.account_id)?;

    Ok(account)
}
