use diesel::prelude::*;
use uuid::Uuid;

use crate::core::schema::authentication_barcode;
use crate::core::{generate_uuid_str, Account, DbConnection, ServiceError, ServiceResult};

/// Represent a barcode - barcode authentication for the given account
#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[table_name = "authentication_barcode"]
#[primary_key(account_id)]
struct AuthenticationBarcode {
    account_id: Uuid,
    code: String,
}

/// Set the barcode as authentication method for the given account
pub fn generate(conn: &DbConnection, account: &Account) -> ServiceResult<()> {
    use crate::core::schema::authentication_barcode::dsl;

    let code = format!("ascii-pay-{}", generate_uuid_str());

    let a = AuthenticationBarcode {
        account_id: account.id,
        code,
    };

    remove(&conn, &account)?;
    diesel::insert_into(dsl::authentication_barcode)
        .values(&a)
        .execute(conn)?;

    Ok(())
}

/// Remove the barcode authentication for the given account
pub fn remove(conn: &DbConnection, account: &Account) -> ServiceResult<()> {
    use crate::core::schema::authentication_barcode::dsl;

    diesel::delete(dsl::authentication_barcode.filter(dsl::account_id.eq(&account.id)))
        .execute(conn)?;

    Ok(())
}

pub fn get_barcodess(conn: &DbConnection, account: &Account) -> ServiceResult<Vec<String>> {
    use crate::core::schema::authentication_barcode::dsl;

    let results = dsl::authentication_barcode
        .filter(dsl::account_id.eq(&account.id))
        .load::<AuthenticationBarcode>(conn)?;

    Ok(results.into_iter().map(|p| p.code).collect())
}

/// Get account by barcode.
/// Return `ServiceError` if no account is registered for given barcode.
pub fn get(conn: &DbConnection, barcode: &str) -> ServiceResult<Account> {
    use crate::core::schema::authentication_barcode::dsl;

    let mut results = dsl::authentication_barcode
        .filter(dsl::code.eq(barcode))
        .limit(1)
        .load::<AuthenticationBarcode>(conn)?;

    let entry = results.pop().ok_or_else(|| ServiceError::NotFound)?;

    let a = Account::get(conn, &entry.account_id)?;

    Ok(a)
}
