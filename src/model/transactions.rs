use std::ops::{Add, AddAssign, Sub, SubAssign};

use chrono::{Local, NaiveDateTime};
use diesel::prelude::*;
use lazy_static::__Deref;
use uuid::Uuid;

use crate::model::schema::{transaction, transaction_item};
use crate::model::Account;
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

#[derive(Debug, Clone, Copy, Default)]
struct TransactionHelper {
    price: Money,
    coffee_stamps: i32,
    bottle_stamps: i32,
}

impl AddAssign<&TransactionItemInput> for TransactionHelper {
    fn add_assign(&mut self, rhs: &TransactionItemInput) {
        match rhs.pay_with_stamps {
            StampType::None => {
                self.price += rhs.price;

                match rhs.give_stamps {
                    StampType::None => {}
                    StampType::Coffee => self.coffee_stamps += 1,
                    StampType::Bottle => self.bottle_stamps += 1,
                }
            }
            StampType::Coffee => self.coffee_stamps -= 10,
            StampType::Bottle => self.bottle_stamps -= 10,
        }
    }
}

impl Add<&TransactionItemInput> for TransactionHelper {
    type Output = Self;

    fn add(self, rhs: &TransactionItemInput) -> Self::Output {
        let mut result = self;
        result += rhs;
        result
    }
}

impl SubAssign<&TransactionItemInput> for TransactionHelper {
    fn sub_assign(&mut self, rhs: &TransactionItemInput) {
        match rhs.pay_with_stamps {
            StampType::None => {
                self.price -= rhs.price;

                match rhs.give_stamps {
                    StampType::None => {}
                    StampType::Coffee => self.coffee_stamps -= 1,
                    StampType::Bottle => self.bottle_stamps -= 1,
                }
            }
            StampType::Coffee => self.coffee_stamps += 10,
            StampType::Bottle => self.bottle_stamps += 10,
        }
    }
}

impl Sub<&TransactionItemInput> for TransactionHelper {
    type Output = Self;

    fn sub(self, rhs: &TransactionItemInput) -> Self::Output {
        let mut result = self;
        result -= rhs;
        result
    }
}

impl TransactionHelper {
    pub fn assert_could_item_be_paid_with_stamps(
        &self,
        account: &Account,
        item: &TransactionItemInput,
    ) -> ServiceResult<()> {
        if account.use_digital_stamps {
            match item.could_be_paid_with_stamps {
                StampType::None => {}
                StampType::Coffee => {
                    if account.coffee_stamps + self.coffee_stamps >= 10 {
                        return Err(ServiceError::TransactionCancelled(
                            "Payment with coffee stamps is possible!".to_owned(),
                        ));
                    }
                }
                StampType::Bottle => {
                    if account.bottle_stamps + self.bottle_stamps >= 10 {
                        return Err(ServiceError::TransactionCancelled(
                            "Payment with bottle stamps is possible!".to_owned(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn assert_can_be_applied_to_account(&self, account: &Account) -> ServiceResult<()> {
        if account.credit + self.price < account.minimum_credit {
            return Err(ServiceError::TransactionError(
                "Insufficient credit!".to_owned(),
            ));
        }

        if account.use_digital_stamps {
            if account.coffee_stamps + self.coffee_stamps < 0 {
                return Err(ServiceError::TransactionError(
                    "Insufficient coffee stamps!".to_owned(),
                ));
            }
            if account.bottle_stamps + self.bottle_stamps < 0 {
                return Err(ServiceError::TransactionError(
                    "Insufficient bottle stamps!".to_owned(),
                ));
            }
        }

        Ok(())
    }

    pub fn apply_to_account(&self, account: &mut Account, date: NaiveDateTime) -> Transaction {
        let old_account = account.clone();

        account.credit += self.price;
        if account.use_digital_stamps {
            account.coffee_stamps += self.coffee_stamps;
            account.bottle_stamps += self.bottle_stamps;
        }

        Transaction {
            id: generate_uuid(),
            account_id: account.id,
            total: self.price,
            before_credit: old_account.credit,
            after_credit: account.credit,
            coffee_stamps: self.coffee_stamps,
            before_coffee_stamps: old_account.coffee_stamps,
            after_coffee_stamps: account.coffee_stamps,
            bottle_stamps: self.bottle_stamps,
            before_bottle_stamps: old_account.bottle_stamps,
            after_bottle_stamps: account.bottle_stamps,
            date,
        }
    }
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

        let mut transaction_helper = TransactionHelper::default();

        // Update credit and stamp values
        for item in transaction_items.iter() {
            transaction_helper += item;
        }

        transaction_helper.assert_can_be_applied_to_account(&account)?;

        // Return error if an item could be paid with stemps
        if stop_if_stamp_payment_is_possible && account.use_digital_stamps {
            for item in transaction_items.iter() {
                let temp = transaction_helper - item;

                temp.assert_could_item_be_paid_with_stamps(&account, item)?;
            }
        }

        let t = transaction_helper.apply_to_account(&mut account, date);

        account.update_sync(database_conn)?;
        diesel::insert_into(dsl::transaction)
            .values(&t)
            .execute(database_conn.deref())?;

        for (i, item) in transaction_items.into_iter().enumerate() {
            let ti = TransactionItem {
                transaction_id: t.id,
                index: i as i32,
                price: item.price,
                pay_with_stamps: item.pay_with_stamps,
                give_stamps: item.give_stamps,
                product_id: item.product_id,
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
