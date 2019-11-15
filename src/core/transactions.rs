use chrono::NaiveDateTime;
use diesel::prelude::*;
use std::collections::HashMap;
use std::convert::TryFrom;

use crate::core::schema::transaction;
use crate::core::{
    generate_uuid, Account, DbConnection, Money, Product, ServiceError, ServiceResult,
};

/// Represent a transaction
#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[table_name = "transaction"]
pub struct Transaction {
    pub id: String,
    pub account: String,
    pub cashier: Option<String>,
    pub total: Money,
    pub date: NaiveDateTime,
}

/// Execute a transaction on the given `account` with the given `total`
///
/// # Internal steps
/// * 1 Start a sql transaction
/// * 2 Requery the account credit
/// * 3 Calculate the new credit
/// * 4 Check if the account limit allows the new credit
/// * 5 Create and save the transaction (with optional cashier refernece)
/// * 6 Save the new credit to the account
pub fn execute(
    conn: &DbConnection,
    account: &mut Account,
    cashier: Option<&Account>,
    total: Money,
) -> ServiceResult<Transaction> {
    use crate::core::schema::transaction::dsl;

    let mut new_credit = account.credit;

    let result = conn.exclusive_transaction(|| {
        let mut account = Account::get(conn, &account.id)?;
        new_credit = account.credit + total;

        if new_credit < account.limit && new_credit < account.credit {
            return Err(ServiceError::InternalServerError(
                "Transaction error",
                "The transaction can not be performed. Check the account credit and limit"
                    .to_owned(),
            ));
        }

        let a = Transaction {
            id: generate_uuid(),
            account: account.id.clone(),
            cashier: cashier.map(|c| c.id.clone()),
            total,
            date: chrono::Local::now().naive_local(),
        };
        account.credit = new_credit;

        diesel::insert_into(dsl::transaction)
            .values(&a)
            .execute(conn)?;

        account.update(conn)?;

        Ok(a)
    });

    if result.is_ok() {
        account.credit = new_credit;
    }

    result
}

// Pagination reference: https://github.com/diesel-rs/diesel/blob/v1.3.0/examples/postgres/advanced-blog-cli/src/pagination.rs
/// List all transactions of a account between the given datetimes
pub fn get_by_account(
    conn: &DbConnection,
    account: &Account,
    from: &NaiveDateTime,
    to: &NaiveDateTime,
) -> ServiceResult<Vec<Transaction>> {
    use crate::core::schema::transaction::dsl;

    let results = dsl::transaction
        .filter(
            dsl::account
                .eq(account.id.to_string())
                .and(dsl::date.between(from, to)),
        )
        .load::<Transaction>(conn)?;

    Ok(results)
}

#[derive(Debug)]
pub enum ValidationError {
    Invalid(Money),
    NoData,
}

/// Check if the credit of an account is valid to its transactions
fn validate_account(
    conn: &DbConnection,
    account: &Account,
) -> ServiceResult<Option<ValidationError>> {
    use crate::core::schema::transaction::dsl;

    conn.exclusive_transaction(|| {
        let account = Account::get(conn, &account.id)?;

        let result = dsl::transaction
            .select(diesel::dsl::sum(dsl::total))
            .filter(dsl::account.eq(account.id.to_string()))
            .first::<Option<i64>>(conn)?;

        if let Some(sum) = result {
            let sum = i32::try_from(sum).map_err(|error| {
                ServiceError::InternalServerError("Validation error", format!("{}", error))
            })?;
            if sum == account.credit {
                Ok(None)
            } else {
                Ok(Some(ValidationError::Invalid(sum)))
            }
        } else {
            Ok(Some(ValidationError::NoData))
        }
    })
}

/// List all accounts with validation erros of their credit to their transactions
pub fn validate_all(conn: &DbConnection) -> ServiceResult<HashMap<Account, ValidationError>> {
    let accounts = Account::all(conn)?;

    let map = accounts
        .into_iter()
        .map(|a| {
            let r = validate_account(conn, &a).unwrap_or(Some(ValidationError::NoData));
            (a, r)
        })
        .filter(|(_, r)| r.is_some())
        .map(|(a, r)| (a, r.expect("Filter removes all none values!")))
        .collect::<HashMap<_, _>>();

    Ok(map)
}

impl Transaction {
    /// Assign products with amounts to this transaction
    pub fn add_products(
        &self,
        conn: &DbConnection,
        products: HashMap<Product, i32>,
    ) -> ServiceResult<()> {
        use crate::core::schema::transaction_product::dsl;

        for (product, amount) in products {
            diesel::insert_into(dsl::transaction_product)
                .values((
                    dsl::transaction.eq(&self.id),
                    dsl::product.eq(&product.id),
                    dsl::amount.eq(amount),
                ))
                .execute(conn)?;
        }

        Ok(())
    }

    /// List assigned products with amounts of this transaction
    pub fn get_products(&self, conn: &DbConnection) -> ServiceResult<HashMap<Product, i32>> {
        use crate::core::schema::transaction_product::dsl;

        let results = dsl::transaction_product
            .filter(dsl::transaction.eq(&self.id))
            .load::<(String, String, i32)>(conn)?;

        let mut map = HashMap::new();

        for (_, p, a) in results {
            let product = Product::get(conn, &p)?;
            map.insert(product, a);
        }

        Ok(map)
    }
}
