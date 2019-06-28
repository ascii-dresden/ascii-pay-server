use actix::{Actor, SyncContext};
use chrono::{NaiveDateTime};
use diesel::sqlite::SqliteConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::convert::From;

use crate::schema::{accounts, users};

pub struct DbExecutor(pub Pool<ConnectionManager<SqliteConnection>>);

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
pub struct Account {
    pub id: String,
    pub display: String,
    pub credit: i32,
    pub limit: i32,
    pub created: NaiveDateTime,
    pub updated: NaiveDateTime
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
pub struct User {
    pub id: String,
    pub account : String,
    pub first_name: String,
    pub last_name: String,
    pub mail: String,
    pub password: String,
    pub created: NaiveDateTime,
    pub updated: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlimUser {
    pub user: String,
}

impl From<User> for SlimUser {
    fn from(user: User) -> Self {
        SlimUser { user: user.id }
    }
}
