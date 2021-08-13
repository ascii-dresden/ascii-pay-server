use chrono::{Duration, Local, NaiveDateTime};
use diesel::prelude::*;
use uuid::Uuid;

use crate::core::schema::session;
use crate::core::{generate_uuid, DbConnection, ServiceError, ServiceResult};

use super::Money;

/// Auto logout after `self` minutes of inactivity
const VALIDITY_MINUTES: i64 = 10;

const TRANSACTION_VALIDITY_SECONDS: i64 = 10;

/// Represent a session
#[derive(
    Debug,
    Queryable,
    Insertable,
    Identifiable,
    AsChangeset,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Clone,
)]
#[table_name = "session"]
pub struct Session {
    pub id: Uuid,
    pub account_id: Uuid,
    pub valid_until: NaiveDateTime,
    pub transaction_total: Option<Money>,
}

impl Session {
    /// Create a new session
    pub fn create_default(conn: &DbConnection, account_id: &Uuid) -> ServiceResult<Session> {
        use crate::core::schema::session::dsl;

        Session::cleanup(&conn)?;

        let a = Session {
            id: generate_uuid(),
            account_id: *account_id,
            valid_until: Local::now().naive_local() + Duration::minutes(VALIDITY_MINUTES),
            transaction_total: None,
        };

        diesel::delete(dsl::session.filter(dsl::id.eq(&a.id))).execute(conn)?;
        diesel::insert_into(dsl::session).values(&a).execute(conn)?;

        Ok(a)
    }

    pub fn create_transaction(
        conn: &DbConnection,
        account_id: &Uuid,
        transaction_total: Money,
    ) -> ServiceResult<Session> {
        use crate::core::schema::session::dsl;

        Session::cleanup(&conn)?;

        let a = Session {
            id: generate_uuid(),
            account_id: *account_id,
            valid_until: Local::now().naive_local()
                + Duration::seconds(TRANSACTION_VALIDITY_SECONDS),
            transaction_total: Some(transaction_total),
        };

        diesel::delete(dsl::session.filter(dsl::id.eq(&a.id))).execute(conn)?;
        diesel::insert_into(dsl::session).values(&a).execute(conn)?;

        Ok(a)
    }

    /// Save the current session data to the database
    pub fn update(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::session::dsl;

        diesel::update(dsl::session.find(&self.id))
            .set(self)
            .execute(conn)?;

        Ok(())
    }

    /// Save the current session data to the database
    pub fn refresh(&mut self) {
        self.valid_until = Local::now().naive_local() + Duration::minutes(VALIDITY_MINUTES);
    }

    /// Get an session by the `id`
    pub fn get(conn: &DbConnection, id: &Uuid) -> ServiceResult<Session> {
        use crate::core::schema::session::dsl;

        let mut results = dsl::session.filter(dsl::id.eq(id)).load::<Session>(conn)?;

        let a = results.pop().ok_or(ServiceError::Unauthorized)?;

        if a.valid_until < Local::now().naive_local() {
            a.delete(&conn)?;
            return Err(ServiceError::Unauthorized);
        }

        Ok(a)
    }

    /// Delete and invalidate this session
    pub fn delete(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::session::dsl;

        diesel::delete(dsl::session.filter(dsl::id.eq(&self.id))).execute(conn)?;

        Ok(())
    }

    /// Delete all expired sessions
    pub fn cleanup(conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::session::dsl;

        let now = Local::now().naive_local();

        diesel::delete(dsl::session.filter(dsl::valid_until.lt(&now))).execute(conn)?;

        Ok(())
    }
}
