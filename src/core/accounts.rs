use diesel::prelude::*;

use crate::core::{Error, DbConnection, Money, generate_uuid};
use crate::core::schema::account;

#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[table_name="account"]
pub struct Account {
    pub id: String,
    pub credit: Money,
    pub limit: Money,
    pub name: Option<String>,
    pub mail: Option<String>
}

impl Account {
    pub fn create(conn: &DbConnection) -> Result<Account, Error> {
        use crate::core::schema::account::dsl::*;

        let a = Account {
            id: generate_uuid(),
            credit: 0,
            limit: 0,
            name: None,
            mail: None
        };

        diesel::insert_into(account)
            .values(&a)
            .execute(conn)?;

        Ok(a)
    }

    pub fn update(&self, conn: &DbConnection) -> Result<(),Error> {
        use crate::core::schema::account::dsl::*;

        diesel::update(account)
            .set(self)
            .execute(conn)?;

        Ok(())
    }

    pub fn all(conn: &DbConnection) -> Result<Vec<Account>, Error> {
        use crate::core::schema::account::dsl::*;

        let results = account
            .load::<Account>(conn)?;

        Ok(results)
    }

    pub fn get(conn: &DbConnection, account_id: &str) -> Result<Account, Error> {
        use crate::core::schema::account::dsl::*;

        let mut results = account
            .filter(id.eq(account_id))
            .load::<Account>(conn)?;

        let a = results.pop().ok_or_else(|| Error::NotFound)?;

        Ok(a)
    }
}
