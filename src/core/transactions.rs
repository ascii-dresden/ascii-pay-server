use chrono::{Local, NaiveDateTime};
use diesel::prelude::*;
use std::collections::HashMap;
use std::convert::TryFrom;
use uuid::Uuid;

use crate::core::schema::transaction;
use crate::core::{
    generate_uuid, Account, DbConnection, Money, Product, ServiceError, ServiceResult,
};

/// Represent a transaction
#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset, Serialize, Deserialize, Clone)]
#[table_name = "transaction"]
pub struct Transaction {
    pub id: Uuid,
    pub account_id: Uuid,
    pub cashier_id: Option<Uuid>,
    pub total: Money,
    pub before_credit: Money,
    pub after_credit: Money,
    pub date: NaiveDateTime,
}

/// Execute a transaction on the given `account` with the given `total`
///
/// # Internal steps
/// * 1 Start a sql transaction
/// * 2 Requery the account credit
/// * 3 Calculate the new credit
/// * 4 Check if the account minimum_credit allows the new credit
/// * 5 Create and save the transaction (with optional cashier refernece)
/// * 6 Save the new credit to the account
fn execute_at(
    conn: &DbConnection,
    account: &mut Account,
    cashier: Option<&Account>,
    total: Money,
    date: NaiveDateTime,
) -> ServiceResult<Transaction> {
    use crate::core::schema::transaction::dsl;

    // TODO: Are empty transaction useful? You can still assign products
    /*
    if total == 0 {
        return Err(ServiceError::BadRequest(
            "Empty transaction",
            "Cannot perform a transaction with a total of zero".to_owned()
        ))
    }
    */

    let before_credit = account.credit;
    let mut after_credit = account.credit;

    let result = conn.build_transaction().serializable().run(|| {
        let mut account = Account::get(conn, &account.id)?;
        after_credit = account.credit + total;

        if after_credit < account.minimum_credit && after_credit < account.credit {
            return Err(ServiceError::InternalServerError(
                "Transaction error",
                "The transaction can not be performed. Check the account credit and minimum_credit"
                    .to_owned(),
            ));
        }

        let a = Transaction {
            id: generate_uuid(),
            account_id: account.id,
            cashier_id: cashier.map(|c| c.id),
            total,
            before_credit,
            after_credit,
            date,
        };
        account.credit = after_credit;

        diesel::insert_into(dsl::transaction)
            .values(&a)
            .execute(conn)?;

        account.update(conn)?;

        Ok(a)
    });

    if result.is_ok() {
        account.credit = after_credit;
    }

    result
}

/// Execute a transaction on the given `account` with the given `total`
///
/// # Internal steps
/// * 1 Start a sql transaction
/// * 2 Requery the account credit
/// * 3 Calculate the new credit
/// * 4 Check if the account minimum_credit allows the new credit
/// * 5 Create and save the transaction (with optional cashier refernece)
/// * 6 Save the new credit to the account
pub fn execute(
    conn: &DbConnection,
    account: &mut Account,
    cashier: Option<&Account>,
    total: Money,
) -> ServiceResult<Transaction> {
    execute_at(conn, account, cashier, total, Local::now().naive_local())
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
            dsl::account_id
                .eq(&account.id)
                .and(dsl::date.between(from, to)),
        )
        .order(dsl::date.desc())
        .load::<Transaction>(conn)?;

    Ok(results)
}

pub fn get_by_account_and_id(
    conn: &DbConnection,
    account: &Account,
    id: &Uuid,
) -> ServiceResult<Transaction> {
    use crate::core::schema::transaction::dsl;

    let mut results = dsl::transaction
        .filter(dsl::account_id.eq(&account.id).and(dsl::id.eq(id)))
        .load::<Transaction>(conn)?;

    results.pop().ok_or_else(|| ServiceError::NotFound)
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

    conn.build_transaction().serializable().run(|| {
        let account = Account::get(conn, &account.id)?;

        let result = dsl::transaction
            .select(diesel::dsl::sum(dsl::total))
            .filter(dsl::account_id.eq(&account.id))
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
        products: Vec<(Product, i32)>,
    ) -> ServiceResult<()> {
        use crate::core::schema::transaction_product::dsl;

        let current_products = self
            .get_products(&conn)?
            .into_iter()
            .collect::<HashMap<Product, i32>>();

        for (product, amount) in products {
            match current_products.get(&product) {
                Some(current_amount) => {
                    diesel::update(
                        dsl::transaction_product.filter(
                            dsl::transaction
                                .eq(&self.id)
                                .and(dsl::product_id.eq(&product.id)),
                        ),
                    )
                    .set(dsl::amount.eq(current_amount + amount))
                    .execute(conn)?;
                }
                None => {
                    diesel::insert_into(dsl::transaction_product)
                        .values((
                            dsl::transaction.eq(&self.id),
                            dsl::product_id.eq(&product.id),
                            dsl::amount.eq(amount),
                        ))
                        .execute(conn)?;
                }
            }
        }

        Ok(())
    }

    /// Remove products with amounts from this transaction
    pub fn remove_products(
        &self,
        conn: &DbConnection,
        products: Vec<(Product, i32)>,
    ) -> ServiceResult<()> {
        use crate::core::schema::transaction_product::dsl;

        let current_products = self
            .get_products(&conn)?
            .into_iter()
            .collect::<HashMap<Product, i32>>();

        for (product, amount) in products {
            if let Some(current_amount) = current_products.get(&product) {
                if *current_amount <= amount {
                    diesel::delete(
                        dsl::transaction_product.filter(
                            dsl::transaction
                                .eq(&self.id)
                                .and(dsl::product_id.eq(&product.id)),
                        ),
                    )
                    .execute(conn)?;
                } else {
                    diesel::update(
                        dsl::transaction_product.filter(
                            dsl::transaction
                                .eq(&self.id)
                                .and(dsl::product_id.eq(&product.id)),
                        ),
                    )
                    .set(dsl::amount.eq(current_amount - amount))
                    .execute(conn)?;
                }
            }
        }

        Ok(())
    }

    /// List assigned products with amounts of this transaction
    pub fn get_products(&self, conn: &DbConnection) -> ServiceResult<Vec<(Product, i32)>> {
        use crate::core::schema::transaction_product::dsl;

        Ok(dsl::transaction_product
            .filter(dsl::transaction.eq(&self.id))
            .load::<(Uuid, Uuid, i32)>(conn)?
            .into_iter()
            .filter_map(|(_, p, a)| match Product::get(conn, &p) {
                Ok(p) => Some((p, a)),
                _ => None,
            })
            .collect())
    }

    pub fn all(conn: &DbConnection) -> ServiceResult<Vec<Transaction>> {
        use crate::core::schema::transaction::dsl;

        let results = dsl::transaction
            .order(dsl::date.desc())
            .load::<Transaction>(conn)?;

        Ok(results)
    }
}

pub fn generate_transactions(
    conn: &DbConnection,
    account: &mut Account,
    from: NaiveDateTime,
    to: NaiveDateTime,
    count_per_day: u32,
    avg_down: Money,
    avg_up: Money,
) -> ServiceResult<()> {
    use chrono::Duration;
    use chrono::NaiveTime;

    let days = (to - from).num_days();
    let start_date = from.date();

    for day_offset in 0..days {
        let offset = Duration::days(day_offset);
        let date = start_date + offset;

        for time_offset in 0..count_per_day {
            let offset = 9.0 / ((count_per_day - 1) as f32) * time_offset as f32;

            let hr = offset as u32;
            let mn = ((offset - hr as f32) * 60.0) as u32;

            let time = NaiveTime::from_hms(hr, mn, 0);

            let date_time = NaiveDateTime::new(date, time);

            let mut seconds = 0;

            while account.credit + avg_down < account.minimum_credit {
                println!("up");
                execute_at(conn, account, None, avg_up, date_time + Duration::seconds(seconds))?;
                seconds += 1;
            }
            println!("down");
            execute_at(conn, account, None, avg_down, date_time + Duration::seconds(seconds))?;
        }
    }

    Ok(())
}
