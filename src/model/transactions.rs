use chrono::{Local, NaiveDateTime};
use diesel::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use crate::model::schema::transaction;
use crate::model::{Account, Product};
use crate::utils::{generate_uuid, DatabaseConnection, Money, ServiceError, ServiceResult};

/// Represent a transaction
#[derive(
    Debug, Queryable, Insertable, Identifiable, AsChangeset, Serialize, Deserialize, Clone,
)]
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
    database_conn: &DatabaseConnection,
    account: &mut Account,
    cashier: Option<&Account>,
    total: Money,
    date: NaiveDateTime,
) -> ServiceResult<Transaction> {
    use crate::model::schema::transaction::dsl;

    let before_credit = account.credit;
    let mut after_credit = account.credit;

    let result = database_conn.build_transaction().serializable().run(|| {
        let mut account = Account::get(database_conn, account.id)?;
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
            .execute(database_conn)?;

        account.update(database_conn)?;

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
    database_conn: &DatabaseConnection,
    account: &mut Account,
    cashier: Option<&Account>,
    total: Money,
) -> ServiceResult<Transaction> {
    let transaction = execute_at(
        database_conn,
        account,
        cashier,
        total,
        Local::now().naive_local(),
    )?;

    Ok(transaction)
}

// Pagination reference: https://github.com/diesel-rs/diesel/blob/v1.3.0/examples/postgres/advanced-blog-cli/src/pagination.rs
/// List all transactions of a account between the given datetimes
pub fn get_by_account(
    database_conn: &DatabaseConnection,
    account: &Account,
    from: Option<NaiveDateTime>,
    to: Option<NaiveDateTime>,
) -> ServiceResult<Vec<Transaction>> {
    use crate::model::schema::transaction::dsl;

    let results = match from {
        Some(f) => match to {
            Some(t) => dsl::transaction
                .filter(dsl::account_id.eq(&account.id).and(dsl::date.between(f, t)))
                .order(dsl::date.desc())
                .load::<Transaction>(database_conn)?,
            None => dsl::transaction
                .filter(dsl::account_id.eq(&account.id).and(dsl::date.ge(f)))
                .order(dsl::date.desc())
                .load::<Transaction>(database_conn)?,
        },
        None => match to {
            Some(t) => dsl::transaction
                .filter(dsl::account_id.eq(&account.id).and(dsl::date.le(t)))
                .order(dsl::date.desc())
                .load::<Transaction>(database_conn)?,
            None => dsl::transaction
                .filter(dsl::account_id.eq(&account.id))
                .order(dsl::date.desc())
                .load::<Transaction>(database_conn)?,
        },
    };

    Ok(results)
}

pub fn get_by_account_and_id(
    database_conn: &DatabaseConnection,
    account: &Account,
    transaction_id: Uuid,
) -> ServiceResult<Transaction> {
    use crate::model::schema::transaction::dsl;

    let mut results = dsl::transaction
        .filter(
            dsl::account_id
                .eq(&account.id)
                .and(dsl::id.eq(transaction_id)),
        )
        .load::<Transaction>(database_conn)?;

    results.pop().ok_or(ServiceError::NotFound)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "status")]
pub enum ValidationResult {
    Ok,
    InvalidTransactionBefore {
        expected_credit: Money,
        transaction_credit: Money,
        transaction: Transaction,
    },
    InvalidTransactionAfter {
        expected_credit: Money,
        transaction_credit: Money,
        transaction: Transaction,
    },
    InvalidSum {
        expected: Money,
        actual: Money,
    },
    NoData,
    Error,
}

/// Check if the credit of an account is valid to its transactions
fn validate_account(
    database_conn: &DatabaseConnection,
    account: &Account,
) -> ServiceResult<ValidationResult> {
    use crate::model::schema::transaction::dsl;

    database_conn.build_transaction().serializable().run(|| {
        let account = Account::get(database_conn, account.id)?;

        let results = dsl::transaction
            .filter(dsl::account_id.eq(&account.id))
            .order(dsl::date.asc())
            .load::<Transaction>(database_conn)?;

        let mut last_credit = 0;

        for transaction in results {
            if last_credit != transaction.before_credit {
                return Ok(ValidationResult::InvalidTransactionBefore {
                    expected_credit: last_credit,
                    transaction_credit: transaction.before_credit,
                    transaction,
                });
            }
            if transaction.before_credit + transaction.total != transaction.after_credit {
                return Ok(ValidationResult::InvalidTransactionAfter {
                    expected_credit: last_credit + transaction.total,
                    transaction_credit: transaction.after_credit,
                    transaction,
                });
            }
            last_credit += transaction.total;
        }

        if last_credit != account.credit {
            Ok(ValidationResult::InvalidSum {
                expected: last_credit,
                actual: account.credit,
            })
        } else {
            Ok(ValidationResult::Ok)
        }
    })
}

/// List all accounts with validation erros of their credit to their transactions
pub fn validate_all(
    database_conn: &DatabaseConnection,
) -> ServiceResult<HashMap<Uuid, ValidationResult>> {
    let accounts = Account::all(database_conn)?;

    let map = accounts
        .into_iter()
        .map(|a| {
            let r = validate_account(database_conn, &a).unwrap_or(ValidationResult::Error);
            (a.id, r)
        })
        .collect::<HashMap<_, _>>();

    Ok(map)
}

impl Transaction {
    /// Assign products with amounts to this transaction
    pub fn add_products(
        &self,
        database_conn: &DatabaseConnection,
        products: Vec<(Product, i32)>,
    ) -> ServiceResult<()> {
        use crate::model::schema::transaction_product::dsl;

        let current_products = self
            .get_products(database_conn)?
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
                    .execute(database_conn)?;
                }
                None => {
                    diesel::insert_into(dsl::transaction_product)
                        .values((
                            dsl::transaction.eq(&self.id),
                            dsl::product_id.eq(&product.id),
                            dsl::amount.eq(amount),
                        ))
                        .execute(database_conn)?;
                }
            }
        }

        Ok(())
    }

    /// Remove products with amounts from this transaction
    pub fn remove_products(
        &self,
        database_conn: &DatabaseConnection,
        products: Vec<(Product, i32)>,
    ) -> ServiceResult<()> {
        use crate::model::schema::transaction_product::dsl;

        let current_products = self
            .get_products(database_conn)?
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
                    .execute(database_conn)?;
                } else {
                    diesel::update(
                        dsl::transaction_product.filter(
                            dsl::transaction
                                .eq(&self.id)
                                .and(dsl::product_id.eq(&product.id)),
                        ),
                    )
                    .set(dsl::amount.eq(current_amount - amount))
                    .execute(database_conn)?;
                }
            }
        }

        Ok(())
    }

    /// List assigned products with amounts of this transaction
    pub fn get_products(
        &self,
        database_conn: &DatabaseConnection,
    ) -> ServiceResult<Vec<(Product, i32)>> {
        use crate::model::schema::transaction_product::dsl;

        Ok(dsl::transaction_product
            .filter(dsl::transaction.eq(&self.id))
            .load::<(Uuid, Uuid, i32)>(database_conn)?
            .into_iter()
            .filter_map(|(_, p, a)| match Product::get(database_conn, p) {
                Ok(p) => Some((p, a)),
                _ => None,
            })
            .collect())
    }

    pub fn all(database_conn: &DatabaseConnection) -> ServiceResult<Vec<Transaction>> {
        use crate::model::schema::transaction::dsl;

        let results = dsl::transaction
            .order(dsl::date.desc())
            .load::<Transaction>(database_conn)?;

        Ok(results)
    }
}
