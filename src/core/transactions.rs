use chrono::NaiveDateTime;
use diesel::prelude::*;
use std::collections::HashMap;
use std::convert::TryFrom;

use crate::core::schema::transaction;
use crate::core::Product;
use crate::core::{generate_uuid, Account, DbConnection, Money, ServiceError};

#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[table_name = "transaction"]
pub struct Transaction {
    pub id: String,
    pub account: String,
    pub cashier: Option<String>,
    pub total: Money,
    pub date: NaiveDateTime,
}

pub fn execute(
    conn: &DbConnection,
    account: &mut Account,
    cashier: Option<&Account>,
    total: Money,
) -> Result<Transaction, ServiceError> {
    use crate::core::schema::transaction::dsl;

    let new_credit = account.credit + total;

    let result = conn.exclusive_transaction(|| {
        let mut account = Account::get(conn, &account.id)?;

        if new_credit < account.limit && new_credit < account.credit {
            return Err(ServiceError::InternalServerError);
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
pub fn get_by_user(
    conn: &DbConnection,
    account: &Account,
    from: &NaiveDateTime,
    to: &NaiveDateTime,
) -> Result<Vec<Transaction>, ServiceError> {
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

fn validate_account(
    conn: &DbConnection,
    account: &Account,
) -> Result<Option<ValidationError>, ServiceError> {
    use crate::core::schema::transaction::dsl;

    conn.exclusive_transaction(|| {
        let account = Account::get(conn, &account.id)?;

        let result = dsl::transaction
            .select(diesel::dsl::sum(dsl::total))
            .filter(dsl::account.eq(account.id.to_string()))
            .first::<Option<i64>>(conn)?;

        if let Some(sum) = result {
            let sum = i32::try_from(sum).map_err(|_| ServiceError::InternalServerError)?;
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

pub fn validate_all(
    conn: &DbConnection,
) -> Result<HashMap<Account, ValidationError>, ServiceError> {
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
    pub fn add_products(
        &self,
        conn: &DbConnection,
        products: HashMap<Product, i32>,
    ) -> Result<(), ServiceError> {
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

    pub fn get_products(&self, conn: &DbConnection) -> Result<HashMap<Product, i32>, ServiceError> {
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
