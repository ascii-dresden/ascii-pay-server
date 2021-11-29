use chrono::{Local, NaiveDateTime};
use diesel::prelude::*;
use lazy_static::__Deref;
use uuid::Uuid;

use crate::model::schema::{transaction, transaction_item};
use crate::model::{Account, Product};
use crate::utils::{
    generate_uuid, DatabaseConnection, DatabasePool, Money, ServiceError, ServiceResult,
};

use super::enums::StampType;

/// Represent a transaction
#[derive(
    Debug, Queryable, Insertable, Identifiable, AsChangeset, Serialize, Deserialize, Clone,
)]
#[table_name = "transaction"]
pub struct Transaction {
    pub id: Uuid,
    pub account_id: Uuid,
    pub total: Money,
    pub before_credit: Money,
    pub after_credit: Money,
    pub coffee_stamps: i32,
    pub before_coffee_stamps: i32,
    pub after_coffee_stamps: i32,
    pub bottle_stamps: i32,
    pub before_bottle_stamps: i32,
    pub after_bottle_stamps: i32,
    pub date: NaiveDateTime,
}

/// Represent a transaction
#[derive(Debug, Queryable, Insertable, AsChangeset, Serialize, Deserialize, Clone)]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "transaction_item"]
pub struct TransactionItem {
    pub transaction_id: Uuid,
    pub index: i32,
    pub price: Money,
    pub pay_with_stamps: StampType,
    pub give_stamps: StampType,
    pub product_id: String,
}

#[derive(Debug, Clone)]
pub struct TransactionItemInput {
    pub price: Money,
    pub pay_with_stamps: StampType,
    pub could_be_paid_with_stamps: StampType,
    pub give_stamps: StampType,
    pub product_id: String,
}

/// Execute a transaction on the given `account` with the given `total`
///
/// # Internal steps
/// * 1 Start a sql transaction
/// * 2 Requery the account credit
/// * 3 Calculate the new credit
/// * 4 Check if the account minimum_credit allows the new credit
/// * 5 Create and save the transaction
/// * 6 Save the new credit to the account
pub async fn execute_at(
    database_pool: &DatabasePool,
    account: &mut Account,
    transaction_items: Vec<TransactionItemInput>,
    stop_if_stamp_payment_is_possible: bool,
    date: NaiveDateTime,
) -> ServiceResult<Transaction> {
    use crate::model::schema::transaction::dsl;
    use crate::model::schema::transaction_item::dsl as dsl_item;

    let database_conn = &database_pool.get().await?;
    let result = database_conn.build_transaction().serializable().run(|| {
        let mut account = Account::get_sync(database_conn, account.id)?;

        let before_credit = account.credit;
        let mut after_credit = account.credit;

        let before_coffee_stamps = account.coffee_stamps;
        let mut after_coffee_stamps = account.coffee_stamps;

        let before_bottle_stamps = account.bottle_stamps;
        let mut after_bottle_stamps = account.bottle_stamps;

        // Update credit and stamp values
        for item in transaction_items.iter() {
            // Update credit if item is not paid with stemps
            if item.pay_with_stamps.is_none() || !account.use_digital_stamps {
                after_credit += item.price;
            }

            // An transaction item cannot give stemps and be paid with stamps at the same time
            if !item.pay_with_stamps.is_none() && !item.give_stamps.is_none() {
                return Err(ServiceError::TransactionError(
                    "An item cannot give stemps and be paid with stamps at the same time!"
                        .to_owned(),
                ));
            }

            if account.use_digital_stamps {
                // Remove 10 stemps if paid with stemps
                match item.pay_with_stamps {
                    StampType::Coffee => {
                        after_coffee_stamps -= 10;
                    }
                    StampType::Bottle => {
                        after_bottle_stamps -= 10;
                    }
                    _ => {}
                }

                // Add 1 stemp if item gives stemp
                match item.give_stamps {
                    StampType::Coffee => {
                        after_coffee_stamps += 1;
                    }
                    StampType::Bottle => {
                        after_bottle_stamps += 1;
                    }
                    _ => {}
                }
            }
        }

        // Return error if an item could be paid with stemps
        if stop_if_stamp_payment_is_possible && account.use_digital_stamps {
            for item in transaction_items.iter() {
                if !item.product_id.is_empty() {
                    let product = Product::get(&item.product_id)?;

                    match product.pay_with_stamps {
                        StampType::Coffee => {
                            if after_coffee_stamps >= 10 {
                                return Err(ServiceError::TransactionCancelled(
                                    "Payment with coffee stamps is possible!".to_owned(),
                                ));
                            }
                        }
                        StampType::Bottle => {
                            if after_bottle_stamps >= 10 {
                                return Err(ServiceError::TransactionCancelled(
                                    "Payment with bottle stamps is possible!".to_owned(),
                                ));
                            }
                        }
                        _ => {}
                    }
                }

                match item.could_be_paid_with_stamps {
                    StampType::Coffee => {
                        if after_coffee_stamps >= 10 {
                            return Err(ServiceError::TransactionCancelled(
                                "Payment with coffee stamps is possible!".to_owned(),
                            ));
                        }
                    }
                    StampType::Bottle => {
                        if after_bottle_stamps >= 10 {
                            return Err(ServiceError::TransactionCancelled(
                                "Payment with bottle stamps is possible!".to_owned(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        if after_credit < account.minimum_credit && after_credit < account.credit {
            return Err(ServiceError::TransactionError(
                "Insufficient credit!".to_owned(),
            ));
        }

        if account.use_digital_stamps
            && after_coffee_stamps < 0
            && after_coffee_stamps < account.coffee_stamps
        {
            return Err(ServiceError::TransactionError(
                "Insufficient coffee stamps!".to_owned(),
            ));
        }

        if account.use_digital_stamps
            && after_bottle_stamps < 0
            && after_bottle_stamps < account.bottle_stamps
        {
            return Err(ServiceError::TransactionError(
                "Insufficient bottle stamps!".to_owned(),
            ));
        }

        let t = Transaction {
            id: generate_uuid(),
            account_id: account.id,
            total: after_credit - before_credit,
            before_credit,
            after_credit,
            coffee_stamps: after_coffee_stamps - before_coffee_stamps,
            before_coffee_stamps,
            after_coffee_stamps,
            bottle_stamps: after_bottle_stamps - before_bottle_stamps,
            before_bottle_stamps,
            after_bottle_stamps,
            date,
        };
        account.credit = after_credit;
        account.coffee_stamps = after_coffee_stamps;
        account.bottle_stamps = after_bottle_stamps;

        account.update_sync(database_conn)?;
        diesel::insert_into(dsl::transaction)
            .values(&t)
            .execute(database_conn.deref())?;

        for (i, item) in transaction_items.iter().enumerate() {
            let ti = TransactionItem {
                transaction_id: t.id,
                index: i as i32,
                price: item.price,
                pay_with_stamps: item.pay_with_stamps,
                give_stamps: item.give_stamps,
                product_id: item.product_id.clone(),
            };

            diesel::insert_into(dsl_item::transaction_item)
                .values(&ti)
                .execute(database_conn.deref())?;
        }

        Ok((t, account))
    });

    match result {
        Ok((t, a)) => {
            account.credit = a.credit;
            account.coffee_stamps = a.coffee_stamps;
            account.bottle_stamps = a.bottle_stamps;
            Ok(t)
        }
        Err(e) => Err(e),
    }
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
pub async fn execute(
    database_pool: &DatabasePool,
    account: &mut Account,
    transaction_items: Vec<TransactionItemInput>,
    stop_if_stamp_payment_is_possible: bool,
) -> ServiceResult<Transaction> {
    execute_at(
        database_pool,
        account,
        transaction_items,
        stop_if_stamp_payment_is_possible,
        Local::now().naive_local(),
    )
    .await
}

/// List all transactions of a account between the given datetimes
pub async fn get_by_account(
    database_pool: &DatabasePool,
    account: &Account,
    from: Option<NaiveDateTime>,
    to: Option<NaiveDateTime>,
) -> ServiceResult<Vec<Transaction>> {
    use crate::model::schema::transaction::dsl;

    let database_conn = &database_pool.get().await?;
    let results = match from {
        Some(f) => match to {
            Some(t) => dsl::transaction
                .filter(dsl::account_id.eq(&account.id).and(dsl::date.between(f, t)))
                .order(dsl::date.desc())
                .load::<Transaction>(database_conn.deref())?,
            None => dsl::transaction
                .filter(dsl::account_id.eq(&account.id).and(dsl::date.ge(f)))
                .order(dsl::date.desc())
                .load::<Transaction>(database_conn.deref())?,
        },
        None => match to {
            Some(t) => dsl::transaction
                .filter(dsl::account_id.eq(&account.id).and(dsl::date.le(t)))
                .order(dsl::date.desc())
                .load::<Transaction>(database_conn.deref())?,
            None => dsl::transaction
                .filter(dsl::account_id.eq(&account.id))
                .order(dsl::date.desc())
                .load::<Transaction>(database_conn.deref())?,
        },
    };

    Ok(results)
}

pub async fn get_by_account_and_id(
    database_pool: &DatabasePool,
    account: &Account,
    transaction_id: Uuid,
) -> ServiceResult<Transaction> {
    use crate::model::schema::transaction::dsl;

    let database_conn = &database_pool.get().await?;
    let mut results = dsl::transaction
        .filter(
            dsl::account_id
                .eq(&account.id)
                .and(dsl::id.eq(transaction_id)),
        )
        .load::<Transaction>(database_conn.deref())?;

    results.pop().ok_or(ServiceError::NotFound)
}

impl Transaction {
    /// List assigned products with amounts of this transaction
    pub fn get_items(
        &self,
        database_conn: &DatabaseConnection,
    ) -> ServiceResult<Vec<TransactionItem>> {
        use crate::model::schema::transaction_item::dsl;

        Ok(dsl::transaction_item
            .filter(dsl::transaction_id.eq(&self.id))
            .load::<TransactionItem>(database_conn.deref())?)
    }
}
