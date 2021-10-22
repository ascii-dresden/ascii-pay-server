use crate::{
    identity_service::{Identity, IdentityRequire},
    model::{
        session::{create_onetime_session, get_onetime_session, Session},
        transactions::{self, TransactionItem, TransactionItemInput},
        Account, Category, Permission, Product, StampType, Transaction,
    },
    utils::{DatabaseConnection, Money, RedisConnection, ServiceError, ServiceResult},
};

use chrono::NaiveDateTime;
use uuid::Uuid;

use super::{accounts::AccountOutput, ProductOutput};

#[derive(Debug, Deserialize, InputObject)]
pub struct PaymentItemInput {
    pub price: Money,
    pub pay_with_stamps: StampType,
    pub could_be_paid_with_stamps: StampType,
    pub give_stamps: StampType,
    pub product_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, InputObject)]
pub struct PaymentInput {
    pub account_access_token: Session,
    pub stop_if_stamp_payment_is_possible: bool,
    pub transaction_items: Vec<PaymentItemInput>,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct PaymentOutput {
    pub account: AccountOutput,
    pub transaction: Option<TransactionOutput>,
    pub account_access_token: Option<Session>,
    pub error: Option<PaymentOutputError>,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct PaymentOutputError {
    pub message: String,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct TransactionOutput {
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
    pub items: Vec<TransactionItemOutput>,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct TransactionItemOutput {
    pub transaction_id: Uuid,
    pub index: i32,
    pub price: Money,
    pub pay_with_stamps: StampType,
    pub give_stamps: StampType,
    pub product: Option<ProductOutput>,
}

type TransactionWrapper = (
    Transaction,
    Vec<(TransactionItem, Option<(Product, Category)>)>,
);

impl From<TransactionWrapper> for TransactionOutput {
    fn from(entity: TransactionWrapper) -> Self {
        let (transaction, items) = entity;

        Self {
            id: transaction.id,
            account_id: transaction.account_id,
            total: transaction.total,
            before_credit: transaction.before_credit,
            after_credit: transaction.after_credit,
            coffee_stamps: transaction.coffee_stamps,
            before_coffee_stamps: transaction.before_coffee_stamps,
            after_coffee_stamps: transaction.after_coffee_stamps,
            bottle_stamps: transaction.bottle_stamps,
            before_bottle_stamps: transaction.before_bottle_stamps,
            after_bottle_stamps: transaction.after_bottle_stamps,
            date: transaction.date,
            items: items
                .into_iter()
                .map(|(i, o)| TransactionItemOutput {
                    transaction_id: i.transaction_id,
                    index: i.index,
                    price: i.price,
                    pay_with_stamps: i.pay_with_stamps,
                    give_stamps: i.give_stamps,
                    product: o.map(|o| o.into()),
                })
                .collect(),
        }
    }
}

pub fn map_with_result<A, B, F>(vec: Vec<A>, transform: F) -> ServiceResult<Vec<B>>
where
    F: Fn(A) -> ServiceResult<B>,
{
    let mut result = Vec::<B>::with_capacity(vec.len());

    for element in vec.into_iter() {
        result.push(transform(element)?);
    }

    Ok(result)
}

pub fn zip_with_result<A, B, F>(vec: Vec<A>, transform: F) -> ServiceResult<Vec<(A, B)>>
where
    F: Fn(&A) -> ServiceResult<B>,
{
    let mut result = Vec::<(A, B)>::with_capacity(vec.len());

    for element in vec.into_iter() {
        let item = transform(&element)?;

        result.push((element, item));
    }

    Ok(result)
}

pub fn map_transaction_output(
    database_conn: &DatabaseConnection,
    transaction: Transaction,
) -> ServiceResult<TransactionOutput> {
    let items = zip_with_result(transaction.get_items(database_conn)?, |item| {
        match item.product_id {
            Some(id) => Some(Product::get(database_conn, id)).transpose(),
            None => Ok(None),
        }
    })?;

    Ok((transaction, items).into())
}

pub fn transaction_payment(
    database_conn: &DatabaseConnection,
    redis_conn: &mut RedisConnection,
    identity: &Identity,
    input: PaymentInput,
) -> ServiceResult<PaymentOutput> {
    identity.require_cert()?;

    let mut account = get_onetime_session(database_conn, redis_conn, &input.account_access_token)?;
    let result = transactions::execute(
        database_conn,
        &mut account,
        input
            .transaction_items
            .iter()
            .map(|item| TransactionItemInput {
                price: item.price,
                pay_with_stamps: item.pay_with_stamps,
                could_be_paid_with_stamps: item.could_be_paid_with_stamps,
                give_stamps: item.give_stamps,
                product_id: item.product_id,
            })
            .collect(),
        input.stop_if_stamp_payment_is_possible,
    );

    let error = match result {
        Ok(transaction) => {
            return Ok(PaymentOutput {
                account: account.into(),
                transaction: Some(map_transaction_output(database_conn, transaction)?),
                account_access_token: None,
                error: None,
            });
        }
        Err(e) => e,
    };

    if let ServiceError::TransactionCancelled(message) = error {
        let account_access_token = create_onetime_session(redis_conn, &account)?;

        return Ok(PaymentOutput {
            account: account.into(),
            transaction: None,
            account_access_token: Some(account_access_token),
            error: Some(PaymentOutputError { message }),
        });
    }

    Err(error)
}

pub fn get_transactions_by_account(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    account_id: Uuid,
    transaction_filer_from: Option<NaiveDateTime>,
    transaction_filer_to: Option<NaiveDateTime>,
) -> ServiceResult<Vec<TransactionOutput>> {
    identity.require_account(Permission::Member)?;

    let account = Account::get(database_conn, account_id)?;
    let entities = transactions::get_by_account(
        database_conn,
        &account,
        transaction_filer_from,
        transaction_filer_to,
    )?;

    map_with_result(entities, |t| map_transaction_output(database_conn, t))
}

pub fn get_transaction_by_account(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    account_id: Uuid,
    transaction_id: Uuid,
) -> ServiceResult<TransactionOutput> {
    identity.require_account(Permission::Member)?;

    let account = Account::get(database_conn, account_id)?;
    let transaction = transactions::get_by_account_and_id(database_conn, &account, transaction_id)?;

    map_transaction_output(database_conn, transaction)
}

pub fn get_transactions_self(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    transaction_filer_from: Option<NaiveDateTime>,
    transaction_filer_to: Option<NaiveDateTime>,
) -> ServiceResult<Vec<TransactionOutput>> {
    let account = identity.require_account(Permission::Default)?;
    let entities = transactions::get_by_account(
        database_conn,
        &account,
        transaction_filer_from,
        transaction_filer_to,
    )?;

    map_with_result(entities, |t| map_transaction_output(database_conn, t))
}

pub fn get_transaction_self(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    transaction_id: Uuid,
) -> ServiceResult<TransactionOutput> {
    let account = identity.require_account(Permission::Default)?;
    let transaction = transactions::get_by_account_and_id(database_conn, &account, transaction_id)?;

    map_transaction_output(database_conn, transaction)
}
