use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::prelude::*;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::*;
use std::io;
use uuid::Uuid;

use crate::core::schema::account;
use crate::core::{generate_uuid, DbConnection, Money, ServiceError, ServiceResult};

/// Represent a account
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
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "account"]
pub struct Account {
    pub id: Uuid,
    pub credit: Money,
    pub minimum_credit: Money,
    pub name: String,
    pub mail: Option<String>,
    pub username: Option<String>,
    pub account_number: Option<String>,
    pub permission: Permission,
    /// Whether the user want's to receive a monthly report about his/her/* account activities
    pub receives_monthly_report: bool,
}

/// Represents the permission level of an account
#[derive(
    Debug, Copy, Clone, FromSqlRow, AsExpression, Hash, PartialEq, Eq, Serialize, Deserialize,
)]
#[sql_type = "SmallInt"]
pub enum Permission {
    /// default user without the ability to edit anything
    DEFAULT,
    /// ascii member who can perform transactions
    MEMBER,
    /// ascii executive or admin who can do everything
    ADMIN,
}

impl PartialOrd for Permission {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Permission {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.level().cmp(&other.level())
    }
}

impl Permission {
    /// Check if the permission level is `Permission::DEFAULT`
    pub fn is_default(self) -> bool {
        Permission::DEFAULT == self
    }

    /// Check if the permission level is `Permission::MEMBER`
    pub fn is_member(self) -> bool {
        Permission::MEMBER == self
    }

    /// Check if the permission level is `Permission::ADMIN`
    pub fn is_admin(self) -> bool {
        Permission::ADMIN == self
    }

    pub fn level(self) -> u32 {
        match self {
            Permission::DEFAULT => 0,
            Permission::MEMBER => 1,
            Permission::ADMIN => 2,
        }
    }
}

/// For manuel database convertion
impl<DB: Backend> ToSql<SmallInt, DB> for Permission
where
    i16: ToSql<SmallInt, DB>,
{
    fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
    where
        W: io::Write,
    {
        let v = match *self {
            Permission::DEFAULT => 0,
            Permission::MEMBER => 1,
            Permission::ADMIN => 2,
        };
        v.to_sql(out)
    }
}

/// For manuel database convertion
impl<DB: Backend> FromSql<SmallInt, DB> for Permission
where
    i16: FromSql<SmallInt, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let v = i16::from_sql(bytes)?;
        Ok(match v {
            0 => Permission::DEFAULT,
            1 => Permission::MEMBER,
            2 => Permission::ADMIN,
            _ => panic!("'{}' is not a valid permission!", &v),
        })
    }
}

impl Account {
    /// Create a new account with the given permission level
    pub fn create(
        conn: &DbConnection,
        name: &str,
        permission: Permission,
    ) -> ServiceResult<Account> {
        use crate::core::schema::account::dsl;

        let a = Account {
            id: generate_uuid(),
            credit: 0,
            minimum_credit: 0,
            name: name.to_owned(),
            mail: None,
            username: None,
            account_number: None,
            permission,
            receives_monthly_report: false,
        };

        if !a.exist_conficting_account(conn)? {
            return Err(ServiceError::InternalServerError(
                "Conficting account settings",
                "The given account settings conflict with the other existing accounts".to_owned(),
            ));
        }

        diesel::insert_into(dsl::account).values(&a).execute(conn)?;

        Ok(a)
    }

    /// Save the current account data to the database
    pub fn update(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::account::dsl;

        if !self.exist_conficting_account(conn)? {
            return Err(ServiceError::InternalServerError(
                "Conficting account settings",
                "The given account settings conflict with the other existing accounts".to_owned(),
            ));
        }

        diesel::update(dsl::account.find(&self.id))
            .set(self)
            .execute(conn)?;

        Ok(())
    }

    /// List all accounts
    pub fn all(conn: &DbConnection) -> ServiceResult<Vec<Account>> {
        use crate::core::schema::account::dsl;

        let results = dsl::account
            .order(dsl::name.asc())
            .load::<Account>(conn)?;

        Ok(results)
    }

    /// Get an account by the `id`
    pub fn get(conn: &DbConnection, id: &Uuid) -> ServiceResult<Account> {
        use crate::core::schema::account::dsl;

        let mut results = dsl::account.filter(dsl::id.eq(id)).load::<Account>(conn)?;

        results.pop().ok_or_else(|| ServiceError::NotFound)
    }

    /// Get an account by the `id`
    pub fn find_by_login(conn: &DbConnection, login: &str) -> ServiceResult<Account> {
        use crate::core::schema::account::dsl;

        let mut results = match Uuid::parse_str(login) {
            Ok(uuid) => dsl::account
                .filter(dsl::id.eq(uuid))
                .load::<Account>(conn)?,
            Err(_) => dsl::account
                .filter(
                    dsl::mail
                        .eq(login)
                        .or(dsl::username.eq(login))
                        .or(dsl::account_number.eq(login)),
                )
                .load::<Account>(conn)?,
        };

        if results.len() > 1 {
            return Err(ServiceError::NotFound);
        }

        results.pop().ok_or_else(|| ServiceError::NotFound)
    }

    fn exist_conficting_account(&self, conn: &DbConnection) -> ServiceResult<bool> {
        use crate::core::schema::account::dsl;

        if let Some(mail) = &self.mail {
            let results = dsl::account
                .filter(dsl::id.ne(self.id).and(dsl::mail.eq(mail)))
                .load::<Account>(conn)?;
            if !results.is_empty() {
                return Ok(false);
            }
        }

        if let Some(username) = &self.username {
            let results = dsl::account
                .filter(dsl::id.ne(self.id).and(dsl::username.eq(username)))
                .load::<Account>(conn)?;
            if !results.is_empty() {
                return Ok(false);
            }
        }

        if let Some(account_number) = &self.account_number {
            let results = dsl::account
                .filter(
                    dsl::id
                        .ne(self.id)
                        .and(dsl::account_number.eq(account_number)),
                )
                .load::<Account>(conn)?;
            if !results.is_empty() {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
