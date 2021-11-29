use argon2rs::verifier::Encoded;
use chrono::{Duration, Local, NaiveDateTime};
use diesel::prelude::*;
use std::fmt;
use uuid::Uuid;

use crate::model::schema::{authentication_password, authentication_password_invitation};
use crate::utils::{env, generate_uuid_str, ServiceError, ServiceResult};
use crate::utils::{mail, DatabasePool};

use super::Account;

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

pub async fn create_invitation_link(
    database_pool: &DatabasePool,
    account: &Account,
) -> ServiceResult<String> {
    use crate::model::schema::authentication_password_invitation::dsl;

    let a = InvitationLink {
        account_id: account.id,
        link: generate_uuid_str(),
        valid_until: Local::now().naive_local() + Duration::days(1),
    };

    revoke_invitation_link(database_pool, account).await?;
    diesel::insert_into(dsl::authentication_password_invitation)
        .values(&a)
        .execute(&*database_pool.get().await?)?;

    // send invite link if account has an associated mail address
    if !account.mail.is_empty() {
        mail::send_invitation_link(account, &a)?;
    }

    Ok(a.link)
}

pub async fn get_invitation_link(
    database_pool: &DatabasePool,
    account: &Account,
) -> ServiceResult<Option<String>> {
    use crate::model::schema::authentication_password_invitation::dsl;

    let mut results = dsl::authentication_password_invitation
        .filter(dsl::account_id.eq(&account.id))
        .limit(1)
        .load::<InvitationLink>(&*database_pool.get().await?)?;

    Ok(results.pop().map(|i| i.link))
}

pub async fn revoke_invitation_link(
    database_pool: &DatabasePool,
    account: &Account,
) -> ServiceResult<()> {
    use crate::model::schema::authentication_password_invitation::dsl;

    diesel::delete(dsl::authentication_password_invitation.filter(dsl::account_id.eq(&account.id)))
        .execute(&*database_pool.get().await?)?;

    Ok(())
}

pub async fn get_account_by_invitation_link(
    database_pool: &DatabasePool,
    link: &str,
) -> ServiceResult<Account> {
    use crate::model::schema::authentication_password_invitation::dsl;

    let mut results = dsl::authentication_password_invitation
        .filter(dsl::link.eq(link))
        .limit(1)
        .load::<InvitationLink>(&*database_pool.get().await?)?;

    let invitation_link = results.pop();

    match invitation_link {
        Some(invitation_link) => Account::get(database_pool, invitation_link.account_id).await,
        None => Err(ServiceError::InternalServerError(
            "Invalid link",
            "".to_owned(),
        )),
    }
}

/// Set the username and password as authentication method for the given account
pub async fn register(
    database_pool: &DatabasePool,
    account: &Account,
    password: &str,
) -> ServiceResult<()> {
    use crate::model::schema::authentication_password::dsl;

    let a = AuthenticationPassword {
        account_id: account.id,
        password: hash_password(password)?,
    };

    revoke_invitation_link(database_pool, account).await?;

    remove(database_pool, account).await?;
    diesel::insert_into(dsl::authentication_password)
        .values(&a)
        .execute(&*database_pool.get().await?)?;

    Ok(())
}

/// Remove the username -password authentication for the given account
pub async fn remove(database_pool: &DatabasePool, account: &Account) -> ServiceResult<()> {
    use crate::model::schema::authentication_password::dsl;

    diesel::delete(dsl::authentication_password.filter(dsl::account_id.eq(&account.id)))
        .execute(&*database_pool.get().await?)?;

    Ok(())
}

pub async fn has_password(database_pool: &DatabasePool, account: &Account) -> ServiceResult<bool> {
    use crate::model::schema::authentication_password::dsl;

    let results = dsl::authentication_password
        .filter(dsl::account_id.eq(&account.id))
        .load::<AuthenticationPassword>(&*database_pool.get().await?)?;

    Ok(!results.is_empty())
}

/// Get account by username and password.
/// Return `ServiceError` if no account is registered for given username - passoword pair
pub async fn get(
    database_pool: &DatabasePool,
    login: &str,
    password: &str,
) -> ServiceResult<Account> {
    use crate::model::schema::authentication_password::dsl;

    let account = Account::find_by_login(database_pool, login).await?;

    let mut results = dsl::authentication_password
        .filter(dsl::account_id.eq(account.id))
        .load::<AuthenticationPassword>(&*database_pool.get().await?)?;

    let entry = results.pop().ok_or(ServiceError::NotFound)?;

    if !verify(&entry.password, password)? {
        return Err(ServiceError::NotFound);
    }

    Account::get(database_pool, entry.account_id).await
}

pub async fn verify_password(
    database_pool: &DatabasePool,
    account: &Account,
    password: &str,
) -> ServiceResult<bool> {
    use crate::model::schema::authentication_password::dsl;

    let mut results = dsl::authentication_password
        .filter(dsl::account_id.eq(account.id))
        .load::<AuthenticationPassword>(&*database_pool.get().await?)?;

    let entry = results.pop().ok_or(ServiceError::NotFound)?;

    verify(&entry.password, password)
}

/// Create the hash version of a password
fn hash_password(password: &str) -> ServiceResult<String> {
    if password.is_empty() {
        return Err(ServiceError::BadRequest(
            "Empty password",
            "Password should not be empty".to_owned(),
        ));
    }

    let bytes =
        Encoded::default2i(password.as_bytes(), env::PASSWORD_SALT.as_bytes(), b"", b"").to_u8();
    Ok(String::from_utf8(bytes)?)
}

/// Verify a password to its hash version
pub fn verify(hash: &str, password: &str) -> ServiceResult<bool> {
    if password.is_empty() {
        return Ok(false);
    }

    let enc = Encoded::from_u8(hash.as_bytes())?;
    Ok(enc.verify(password.as_bytes()))
}
