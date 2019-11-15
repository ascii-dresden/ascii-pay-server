use argonautica::{Hasher, Verifier};
use diesel::prelude::*;

use crate::core::schema::authentication_password;
use crate::core::{Account, DbConnection, ServiceError, ServiceResult};

/// Represent a username - password authentication for the given account
#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[table_name = "authentication_password"]
#[primary_key(account)]
struct AuthenticationPassword {
    account: String,
    username: String,
    password: String,
}

/// Set the username and password as authentication method for the given account
pub fn register(
    conn: &DbConnection,
    account: &Account,
    username: &str,
    password: &str,
) -> ServiceResult<()> {
    use crate::core::schema::authentication_password::dsl;

    let a = AuthenticationPassword {
        account: account.id.clone(),
        username: username.into(),
        password: hash_password(password)?,
    };

    remove(&conn, &account)?;
    diesel::insert_into(dsl::authentication_password)
        .values(&a)
        .execute(conn)?;

    Ok(())
}

/// Remove the username -password authentication for the given account
pub fn remove(conn: &DbConnection, account: &Account) -> ServiceResult<()> {
    use crate::core::schema::authentication_password::dsl;

    diesel::delete(dsl::authentication_password.filter(dsl::account.eq(&account.id)))
        .execute(conn)?;

    Ok(())
}

/// Get account by username and password.
/// Return `ServiceError` if no account is registered for given username - passoword pair
pub fn get(conn: &DbConnection, username: &str, password: &str) -> ServiceResult<Account> {
    use crate::core::schema::authentication_password::dsl;

    let mut results = dsl::authentication_password
        .filter(dsl::username.eq(username))
        .limit(1)
        .load::<AuthenticationPassword>(conn)?;

    let entry = results.pop().ok_or_else(|| ServiceError::NotFound)?;

    if !verify(&entry.password, password)? {
        return Err(ServiceError::NotFound);
    }

    let a = Account::get(conn, &entry.account)?;

    Ok(a)
}

lazy_static::lazy_static! {
pub  static ref SECRET_KEY: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));
}

/// Create the hash version of a password
fn hash_password(password: &str) -> ServiceResult<String> {
    Hasher::default()
        .with_password(password)
        .with_secret_key(SECRET_KEY.as_str())
        .hash()
        .map_err(|err| {
            dbg!(&err);
            ServiceError::InternalServerError("Hash password", format!("{}", err))
        })
}

/// Verify a password to its hash version
fn verify(hash: &str, password: &str) -> ServiceResult<bool> {
    Verifier::default()
        .with_hash(hash)
        .with_password(password)
        .with_secret_key(SECRET_KEY.as_str())
        .verify()
        .map_err(|err| ServiceError::InternalServerError("Hash password", format!("{}", err)))
}
