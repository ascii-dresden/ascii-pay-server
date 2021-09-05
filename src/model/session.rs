use core::fmt;

use async_graphql::{InputType, InputValueError, InputValueResult, Value};
use serde::de::{Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};
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

#[derive(Debug, Clone, Copy, SimpleObject)]
pub struct Session {
    key: Uuid,
}

impl Session {
    pub fn new() -> Self {
        Self {
            key: generate_uuid(),
        }
    }

    pub fn from_str(s: &str) -> ServiceResult<Self> {
        let obj = parse_obj_from_token::<InnerSession>(s)?;
        Ok(Self { key: obj.key })
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_string(&self) -> ServiceResult<String> {
        create_token_from_obj(&InnerSession { key: self.key })
    }
}

impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(s) = self.to_string() {
            write!(f, "{}", s)
        } else {
            Err(fmt::Error)
        }
    }
}

impl Serialize for Session {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Ok(s) = self.to_string() {
            serializer.serialize_str(&s)
        } else {
            Err(serde::ser::Error::custom("Cannot serialize Session!"))
        }
    }
}
impl InputType for Session {
    fn parse(value: Option<Value>) -> InputValueResult<Self> {
        if let Some(Value::String(s)) = value {
            Self::from_str(&s)
                .map_err(|_| InputValueError::<Self>::custom(format!("Cannot parse session {}", s)))
        } else {
            Err(InputValueError::<Self>::custom("Cannot parse empty session".to_owned()))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.to_string().unwrap_or_default())
    }
}

struct SessionVisitor;
impl<'de> Visitor<'de> for SessionVisitor {
    type Value = Session;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an valid session token")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if let Ok(s) = Session::from_str(value) {
            Ok(s)
        } else {
            Err(serde::de::Error::custom("Cannot serialize Session!"))
        }
    }
}

impl<'de> Deserialize<'de> for Session {
    fn deserialize<D>(deserializer: D) -> Result<Session, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(SessionVisitor)
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
    let session = Session::new();

    redis::create_data::<LongtimeSession>(
        redis_conn,
        &uuid_to_str(session.key),
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
        redis::get_data::<LongtimeSession>(redis_conn, &uuid_to_str(session.key), 10 * 60)?;

    let account = Account::get(database_conn, session.account_id)?;

    Ok(account)
}

pub fn delete_longtime_session(
    redis_conn: &mut RedisConnection,
    session: &Session,
) -> ServiceResult<()> {
    redis::delete_data::<LongtimeSession>(redis_conn, &uuid_to_str(session.key))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct OnetimeSession {
    account_id: Uuid,
}

pub fn create_onetime_session(
    redis_conn: &mut RedisConnection,
    account: &Account,
) -> ServiceResult<Session> {
    let session = Session::new();

    redis::create_data::<OnetimeSession>(
        redis_conn,
        &uuid_to_str(session.key),
        &OnetimeSession {
            account_id: account.id,
        },
        10,
    )?;

    Ok(session)
}

pub fn get_onetime_session(
    database_conn: &DatabaseConnection,
    redis_conn: &mut RedisConnection,
    session: &Session,
) -> ServiceResult<Account> {
    let session = redis::get_delete_data::<OnetimeSession>(redis_conn, &uuid_to_str(session.key))?;

    let account = Account::get(database_conn, session.account_id)?;

    Ok(account)
}
