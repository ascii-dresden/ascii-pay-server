use diesel::prelude::*;
use uuid::Uuid;

use crate::core::schema::authentication_nfc;
use crate::core::{Account, DbConnection, ServiceError, ServiceResult};

/// Represent a nfc tag - nfc authentication for the given account
#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "authentication_nfc"]
#[primary_key(account_id)]
struct AuthenticationNfc {
    account_id: Uuid,
    card_id: String,
    key: Option<String>,
    secret: Option<String>,
}

/// Set the nfc as authentication method for the given account
pub fn register(
    conn: &DbConnection,
    account: &Account,
    card_id: &str,
    key: Option<&str>,
    secret: Option<&str>,
) -> ServiceResult<()> {
    use crate::core::schema::authentication_nfc::dsl;

    let a = AuthenticationNfc {
        account_id: account.id,
        card_id: card_id.to_owned(),
        key: key.map(|x| x.to_owned()),
        secret: secret.map(|x| x.to_owned()),
    };

    remove(&conn, &account)?;
    diesel::insert_into(dsl::authentication_nfc)
        .values(&a)
        .execute(conn)?;

    Ok(())
}

/// Remove the nfc authentication for the given account
pub fn remove(conn: &DbConnection, account: &Account) -> ServiceResult<()> {
    use crate::core::schema::authentication_nfc::dsl;

    diesel::delete(dsl::authentication_nfc.filter(dsl::account_id.eq(&account.id)))
        .execute(conn)?;

    Ok(())
}

pub fn get_nfcs(conn: &DbConnection, account: &Account) -> ServiceResult<Vec<String>> {
    use crate::core::schema::authentication_nfc::dsl;

    let results = dsl::authentication_nfc
        .filter(dsl::account_id.eq(&account.id))
        .load::<AuthenticationNfc>(conn)?;

    Ok(results.into_iter().map(|p| p.card_id).collect())
}

pub enum NfcResult {
    Ok { account: Account },
    AuthenticationRequested { key: String },
}

/// Get account by nfc.
/// Return `ServiceError` if no account is registered for given nfc.
pub fn get(conn: &DbConnection, card_id: &str) -> ServiceResult<Account> {
    use crate::core::schema::authentication_nfc::dsl;

    let mut results = dsl::authentication_nfc
        .filter(dsl::card_id.eq(card_id))
        .limit(1)
        .load::<AuthenticationNfc>(conn)?;

    let entry = results.pop().ok_or_else(|| ServiceError::NotFound)?;

    Account::get(conn, &entry.account_id)
}

/// Get account by nfc.
/// Return `ServiceError` if no account is registered for given nfc.
pub fn get_with_secret(
    conn: &DbConnection,
    card_id: &str,
    secret: &str,
) -> ServiceResult<NfcResult> {
    use crate::core::schema::authentication_nfc::dsl;

    let mut results = dsl::authentication_nfc
        .filter(dsl::card_id.eq(card_id))
        .limit(1)
        .load::<AuthenticationNfc>(conn)?;

    let entry = results.pop().ok_or_else(|| ServiceError::NotFound)?;

    if let Some(ref server_secret) = entry.secret {
        if server_secret == secret {
            let account = Account::get(conn, &entry.account_id)?;
            return Ok(NfcResult::Ok { account });
        }
        Err(ServiceError::BadRequest(
            "Wrong secret",
            "The given secret does not match!".to_owned(),
        ))
    } else {
        Err(ServiceError::BadRequest(
            "Illegal secret",
            "No secret is required for this card!".to_owned(),
        ))
    }
}
