use crate::{
    identity_service::{Identity, IdentityRequire},
    model::{
        session::{get_onetime_session, Session},
        transactions, Account, Permission, Product, Transaction,
    },
    utils::{DatabaseConnection, Money, RedisConnection, ServiceResult},
};

use chrono::NaiveDateTime;
use uuid::Uuid;

use super::{accounts::AccountOutput, ProductOutput};

#[derive(Debug, Deserialize, InputObject)]
pub struct PaymentInput {
    pub account_access_token: Session,
    pub amount: i32,
    pub products: Vec<PaymentProductInput>,
}

#[derive(Debug, Deserialize, InputObject)]
pub struct PaymentProductInput {
    pub id: Uuid,
    pub amount: i32,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct PaymentOutput {
    pub account: AccountOutput,
    pub transaction: TransactionOutput,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct TransactionOutput {
    pub id: Uuid,
    pub account_id: Uuid,
    pub cashier_id: Option<Uuid>,
    pub total: Money,
    pub before_credit: Money,
    pub after_credit: Money,
    pub date: NaiveDateTime,
    pub products: Vec<TransactionProductOutput>,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct TransactionProductOutput {
    pub product: ProductOutput,
    pub amount: i32,
}

impl From<Transaction> for TransactionOutput {
    fn from(entity: Transaction) -> Self {
        Self {
            id: entity.id,
            account_id: entity.account_id,
            cashier_id: entity.cashier_id,
            total: entity.total,
            before_credit: entity.before_credit,
            after_credit: entity.after_credit,
            date: entity.date,
            products: Vec::new(),
        }
    }
}

impl From<(Transaction, Vec<(Product, i32)>)> for TransactionOutput {
    fn from(entity: (Transaction, Vec<(Product, i32)>)) -> Self {
        Self {
            id: entity.0.id,
            account_id: entity.0.account_id,
            cashier_id: entity.0.cashier_id,
            total: entity.0.total,
            before_credit: entity.0.before_credit,
            after_credit: entity.0.after_credit,
            date: entity.0.date,
            products: entity
                .1
                .into_iter()
                .map(|(product, amount)| TransactionProductOutput {
                    product: product.into(),
                    amount,
                })
                .collect(),
        }
    }
}

pub fn transaction_payment(
    database_conn: &DatabaseConnection,
    redis_conn: &mut RedisConnection,
    identity: &Identity,
    input: PaymentInput,
) -> ServiceResult<PaymentOutput> {
    identity.require_cert()?;

    let mut account = get_onetime_session(database_conn, redis_conn, &input.account_access_token)?;
    let transaction = transactions::execute(database_conn, &mut account, None, input.amount)?;

    let mut products: Vec<(Product, i32)> = Vec::new();

    for payment_product in &input.products {
        if let Ok(product) = Product::get(database_conn, payment_product.id) {
            products.push((product, payment_product.amount));
        }
    }

    transaction.add_products(database_conn, products)?;

    Ok(PaymentOutput {
        account: account.into(),
        transaction: transaction.into(),
    })
}

pub fn get_transactions_by_account(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    account_id: Uuid,
    transaction_filer_from: Option<NaiveDateTime>,
    transaction_filer_to: Option<NaiveDateTime>,
) -> ServiceResult<Vec<TransactionOutput>> {
    identity.require_account_or_cert(Permission::Member)?;

    let account = Account::get(database_conn, account_id)?;
    let entities = transactions::get_by_account(
        database_conn,
        &account,
        transaction_filer_from,
        transaction_filer_to,
    )?
    .into_iter()
    .map(|t| {
        let p = t.get_products(database_conn).unwrap_or_default();
        (t, p)
    })
    .map(TransactionOutput::from)
    .collect();

    Ok(entities)
}

pub fn get_transaction_by_account(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    account_id: Uuid,
    transaction_id: Uuid,
) -> ServiceResult<TransactionOutput> {
    identity.require_account_or_cert(Permission::Member)?;

    let account = Account::get(database_conn, account_id)?;
    let entity =
        transactions::get_by_account_and_id(database_conn, &account, transaction_id)?.into();

    Ok(entity)
}
