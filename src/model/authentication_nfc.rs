use diesel::prelude::*;
use uuid::Uuid;

use crate::model::schema::authentication_nfc;
use crate::utils::{DatabaseConnection, ServiceError, ServiceResult};

use super::Account;

/// Represent a nfc tag - nfc authentication for the given account
#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "authentication_nfc"]
#[primary_key(account_id, card_id)]
pub struct AuthenticationNfc {
    pub account_id: Uuid,
    pub card_id: String,
    pub card_type: String,
    pub name: String,
    pub data: String,
}

/// Set the nfc as authentication method for the given account
pub fn register(
    database_conn: &DatabaseConnection,
    account: &Account,
    card_id: &str,
    card_type: &str,
    name: &str,
    data: &str,
) -> ServiceResult<()> {
    use crate::model::schema::authentication_nfc::dsl;

    if !account.allow_nfc_registration {
        return Err(ServiceError::Unauthorized);
    }

    let a = AuthenticationNfc {
        account_id: account.id,
        card_id: card_id.to_owned(),
        card_type: card_type.to_owned(),
        name: name.to_owned(),
        data: data.to_owned(),
    };

    remove(database_conn, account)?;
    diesel::insert_into(dsl::authentication_nfc)
        .values(&a)
        .execute(database_conn)?;

    Ok(())
}

/// Remove the nfc authentication for the given account
pub fn remove(database_conn: &DatabaseConnection, account: &Account) -> ServiceResult<()> {
    use crate::model::schema::authentication_nfc::dsl;

    diesel::delete(dsl::authentication_nfc.filter(dsl::account_id.eq(&account.id)))
        .execute(database_conn)?;

    Ok(())
}

pub fn get_by_account(
    database_conn: &DatabaseConnection,
    account: &Account,
) -> ServiceResult<Vec<AuthenticationNfc>> {
    use crate::model::schema::authentication_nfc::dsl;

    let results = dsl::authentication_nfc
        .filter(dsl::account_id.eq(&account.id))
        .load::<AuthenticationNfc>(database_conn)?;

    Ok(results)
}

pub fn get_by_card_id(
    database_conn: &DatabaseConnection,
    card_id: &str,
) -> ServiceResult<AuthenticationNfc> {
    use crate::model::schema::authentication_nfc::dsl;

    let mut results = dsl::authentication_nfc
        .filter(dsl::card_id.eq(&card_id))
        .load::<AuthenticationNfc>(database_conn)?;

    results.pop().ok_or(ServiceError::NotFound)
}
