use argonautica::{Hasher, Verifier};
use chrono::{Duration, Local, NaiveDateTime};
use diesel::prelude::*;
use std::fmt;
use uuid::Uuid;

use crate::core::mail;
use crate::core::schema::{authentication_password, authentication_password_invitation};
use crate::core::{env, generate_uuid_str, Account, DbConnection, ServiceError, ServiceResult};

/// Represent a username - password authentication for the given account
#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[table_name = "authentication_password"]
#[primary_key(account_id)]
struct AuthenticationPassword {
    account_id: Uuid,
    password: String,
}

#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[table_name = "authentication_password_invitation"]
#[primary_key(account_id)]
pub struct InvitationLink {
    pub account_id: Uuid,
    pub link: String,
    pub valid_until: NaiveDateTime,
}

impl fmt::Display for InvitationLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{base}/register/{link_id}",
            base = env::BASE_URL.as_str(),
            link_id = self.link
        )
    }
}

pub fn create_invitation_link(conn: &DbConnection, account: &Account) -> ServiceResult<String> {
    use crate::core::schema::authentication_password_invitation::dsl;

    let a = InvitationLink {
        account_id: account.id,
        link: generate_uuid_str(),
        valid_until: Local::now().naive_local() + Duration::days(1),
    };

    revoke_invitation_link(&conn, &account)?;
    diesel::insert_into(dsl::authentication_password_invitation)
        .values(&a)
        .execute(conn)?;

    // send invite link if account has an associated mail address
    if account.mail.is_some() {
        mail::send_invitation_link(&account, &a)?;
    }

    Ok(a.link)
}

pub fn get_invitation_link(
    conn: &DbConnection,
    account: &Account,
) -> ServiceResult<Option<String>> {
    use crate::core::schema::authentication_password_invitation::dsl;

    let mut results = dsl::authentication_password_invitation
        .filter(dsl::account_id.eq(&account.id))
        .limit(1)
        .load::<InvitationLink>(conn)?;

    Ok(results.pop().map(|i| i.link))
}

pub fn revoke_invitation_link(conn: &DbConnection, account: &Account) -> ServiceResult<()> {
    use crate::core::schema::authentication_password_invitation::dsl;

    diesel::delete(dsl::authentication_password_invitation.filter(dsl::account_id.eq(&account.id)))
        .execute(conn)?;

    Ok(())
}

pub fn get_account_by_invitation_link(conn: &DbConnection, link: &str) -> ServiceResult<Account> {
    use crate::core::schema::authentication_password_invitation::dsl;

    let mut results = dsl::authentication_password_invitation
        .filter(dsl::link.eq(link))
        .limit(1)
        .load::<InvitationLink>(conn)?;

    let invitation_link = results.pop();

    match invitation_link {
        Some(invitation_link) => Account::get(conn, &invitation_link.account_id),
        None => Err(ServiceError::InternalServerError(
            "Invalid link",
            "".to_owned(),
        )),
    }
}

/// Set the username and password as authentication method for the given account
pub fn register(conn: &DbConnection, account: &Account, password: &str) -> ServiceResult<()> {
    use crate::core::schema::authentication_password::dsl;

    let a = AuthenticationPassword {
        account_id: account.id,
        password: hash_password(password)?,
    };

    revoke_invitation_link(&conn, &account)?;

    remove(&conn, &account)?;
    diesel::insert_into(dsl::authentication_password)
        .values(&a)
        .execute(conn)?;

    Ok(())
}

/// Remove the username -password authentication for the given account
pub fn remove(conn: &DbConnection, account: &Account) -> ServiceResult<()> {
    use crate::core::schema::authentication_password::dsl;

    diesel::delete(dsl::authentication_password.filter(dsl::account_id.eq(&account.id)))
        .execute(conn)?;

    Ok(())
}

pub fn has_password(conn: &DbConnection, account: &Account) -> ServiceResult<bool> {
    use crate::core::schema::authentication_password::dsl;

    let results = dsl::authentication_password
        .filter(dsl::account_id.eq(&account.id))
        .load::<AuthenticationPassword>(conn)?;

    Ok(!results.is_empty())
}

/// Get account by username and password.
/// Return `ServiceError` if no account is registered for given username - passoword pair
pub fn get(conn: &DbConnection, login: &str, password: &str) -> ServiceResult<Account> {
    use crate::core::schema::authentication_password::dsl;

    let account = Account::find_by_login(&conn, login)?;

    let mut results = dsl::authentication_password
        .filter(dsl::account_id.eq(account.id))
        .load::<AuthenticationPassword>(conn)?;

    let entry = results.pop().ok_or_else(|| ServiceError::NotFound)?;

    if !verify(&entry.password, password)? {
        return Err(ServiceError::NotFound);
    }

    let a = Account::get(conn, &entry.account_id)?;

    Ok(a)
}

pub fn verify_password(
    conn: &DbConnection,
    account: &Account,
    password: &str,
) -> ServiceResult<bool> {
    use crate::core::schema::authentication_password::dsl;

    let mut results = dsl::authentication_password
        .filter(dsl::account_id.eq(account.id))
        .load::<AuthenticationPassword>(conn)?;

    let entry = results.pop().ok_or_else(|| ServiceError::NotFound)?;

    Ok(verify(&entry.password, password)?)
}

/// Create the hash version of a password
fn hash_password(password: &str) -> ServiceResult<String> {
    if password.is_empty() {
        return Err(ServiceError::BadRequest(
            "Empty password",
            "Password should not be empty".to_owned(),
        ));
    }
    Hasher::default()
        .with_password(password)
        .with_secret_key(env::PASSWORD_SALT.as_str())
        .hash()
        .map_err(|err| {
            dbg!(&err);
            ServiceError::InternalServerError("Hash password", format!("{}", err))
        })
}

/// Verify a password to its hash version
fn verify(hash: &str, password: &str) -> ServiceResult<bool> {
    if password.is_empty() {
        return Ok(false);
    }
    Verifier::default()
        .with_hash(hash)
        .with_password(password)
        .with_secret_key(env::PASSWORD_SALT.as_str())
        .verify()
        .map_err(|err| ServiceError::InternalServerError("Hash password", format!("{}", err)))
}
