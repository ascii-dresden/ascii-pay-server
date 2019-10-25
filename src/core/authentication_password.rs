use diesel::prelude::*;
use argonautica::{Hasher, Verifier};

use crate::core::{Error, DbConnection, Account};
use crate::core::schema::authentication_password;

#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[table_name="authentication_password"]
#[primary_key(account)]
struct AuthenticationPassword {
    account: String,
    username: String,
    password: String
}

pub fn register(conn: &DbConnection, a: &Account, uname: &str, pword: &str) -> Result<(), Error> {
        use crate::core::schema::authentication_password::dsl::*;

        let a = AuthenticationPassword {
            account: a.id.clone(),
            username: uname.into(),
            password: hash_password(pword)?
        };

        diesel::insert_into(authentication_password)
            .values(&a)
            .execute(conn)?;

        Ok(())
}

pub fn remove(conn: &DbConnection, a: &Account) -> Result<(), Error> {
        use crate::core::schema::authentication_password::dsl::*;

        diesel::delete(
            authentication_password.filter(account.eq(&a.id))
            ).execute(conn)?;

        Ok(())
}
pub fn get(conn: &DbConnection, uname: &str, pword: &str) -> Result<Account, Error> {
        use crate::core::schema::authentication_password::dsl::*;

        let mut results = authentication_password
            .filter(username.eq(uname))
            .limit(1)
            .load::<AuthenticationPassword>(conn)?;

        let entry = results.pop().ok_or_else(|| Error::NotFound)?;

        if !verify(&entry.password, pword)? {
            return Err(Error::NotFound);
        }

        let a = Account::get(conn, &entry.account)?;

        Ok(a)
}



lazy_static::lazy_static! {
pub  static ref SECRET_KEY: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));
}


fn hash_password(password: &str) -> Result<String, Error> {
    Hasher::default()
        .with_password(password)
        .with_secret_key(SECRET_KEY.as_str())
        .hash()
        .map_err(|err| {
            dbg!(err);
            Error::InternalServerError
        })
}

fn verify(hash: &str, password: &str) -> Result<bool, Error> {
    Verifier::default()
        .with_hash(hash)
        .with_password(password)
        .with_secret_key(SECRET_KEY.as_str())
        .verify()
        .map_err(|_| {
            Error::InternalServerError
        })
}
