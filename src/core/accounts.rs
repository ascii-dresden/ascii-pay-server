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
    pub name: Option<String>,
    pub mail: Option<String>,
    pub permission: Permission,
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
    pub fn create(conn: &DbConnection, permission: Permission) -> ServiceResult<Account> {
        use crate::core::schema::account::dsl;

        let a = Account {
            id: generate_uuid(),
            credit: 0,
            minimum_credit: 0,
            name: None,
            mail: None,
            permission,
        };

        diesel::insert_into(dsl::account).values(&a).execute(conn)?;

        Ok(a)
    }

    /// Save the current account data to the database
    pub fn update(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::account::dsl;

        diesel::update(dsl::account.find(&self.id))
            .set(self)
            .execute(conn)?;

        Ok(())
    }

    /// List all accounts
    pub fn all(conn: &DbConnection) -> ServiceResult<Vec<Account>> {
        use crate::core::schema::account::dsl;

        let results = dsl::account.load::<Account>(conn)?;

        Ok(results)
    }

    /// Get an account by the `id`
    pub fn get(conn: &DbConnection, id: &Uuid) -> ServiceResult<Account> {
        use crate::core::schema::account::dsl;

        let mut results = dsl::account.filter(dsl::id.eq(id)).load::<Account>(conn)?;

        let a = results.pop().ok_or_else(|| ServiceError::NotFound)?;

        Ok(a)
    }

    pub fn import(conn: &DbConnection, template: &Account) -> ServiceResult<Account> {
        use crate::core::schema::account::dsl;

        let a = Account {
            id: generate_uuid(),
            credit: 0,
            minimum_credit: template.minimum_credit,
            name: template.name.to_owned(),
            mail: template.mail.to_owned(),
            permission: template.permission,
        };

        diesel::insert_into(dsl::account).values(&a).execute(conn)?;

        Ok(a)
    }
}
