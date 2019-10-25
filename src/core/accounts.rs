use diesel::prelude::*;

use crate::core::schema::account;
use crate::core::{generate_uuid, DbConnection, Error, Money};

#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset, PartialEq, Eq, Hash)]
#[table_name = "account"]
pub struct Account {
    pub id: String,
    pub credit: Money,
    pub limit: Money,
    pub name: Option<String>,
    pub mail: Option<String>,
}

impl Account {
    pub fn create(conn: &DbConnection) -> Result<Account, Error> {
        use crate::core::schema::account::dsl;

        let a = Account {
            id: generate_uuid(),
            credit: 0,
            limit: 0,
            name: None,
            mail: None,
        };

        diesel::insert_into(dsl::account).values(&a).execute(conn)?;

        Ok(a)
    }

    pub fn update(&self, conn: &DbConnection) -> Result<(), Error> {
        use crate::core::schema::account::dsl;

        diesel::update(dsl::account).set(self).execute(conn)?;

        Ok(())
    }

    pub fn all(conn: &DbConnection) -> Result<Vec<Account>, Error> {
        use crate::core::schema::account::dsl;

        let results = dsl::account.load::<Account>(conn)?;

        Ok(results)
    }

    pub fn get(conn: &DbConnection, id: &str) -> Result<Account, Error> {
        use crate::core::schema::account::dsl;

        let mut results = dsl::account.filter(dsl::id.eq(id)).load::<Account>(conn)?;

        let a = results.pop().ok_or_else(|| Error::NotFound)?;

        Ok(a)
    }
}
