use crate::{
    identity_service::{Identity, IdentityRequire},
    model::{
        session::{get_onetime_session, Session},
        transactions, Product, Transaction,
    },
    utils::{DatabaseConnection, Money, RedisConnection, ServiceResult},
};

use chrono::NaiveDateTime;
use std::collections::HashMap;
use uuid::Uuid;

use super::accounts::AccountOutput;

#[derive(Debug, Deserialize)]
pub struct PaymentInput {
    pub account_access_token: Session,
    pub amount: i32,
    pub products: HashMap<Uuid, i32>,
}

#[derive(Debug, Serialize)]
pub struct PaymentOutput {
    pub account: AccountOutput,
    pub transaction: TransactionOutput,
}

#[derive(Debug, Serialize)]
pub struct TransactionOutput {
    pub id: Uuid,
    pub account_id: Uuid,
    pub cashier_id: Option<Uuid>,
    pub total: Money,
    pub before_credit: Money,
    pub after_credit: Money,
    pub date: NaiveDateTime,
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

    for (product_id, amount) in &input.products {
        if let Ok(product) = Product::get(database_conn, product_id) {
            products.push((product, *amount));
        }
    }

    transaction.add_products(database_conn, products)?;

    Ok(PaymentOutput {
        account: account.into(),
        transaction: transaction.into(),
    })
}
