use crate::{
    identity_service::{Identity, IdentityRequire},
    model::{
        authentication_barcode, authentication_nfc, create_token_from_obj, parse_obj_from_token,
        transactions, Account, DbConnection, Money, Product, ServiceError, ServiceResult, Session,
        Transaction,
    },
};

use chrono::NaiveDateTime;
use std::collections::HashMap;
use uuid::Uuid;

use super::accounts::AccountOutput;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum AuthenticationInput {
    Barcode {
        code: String,
    },
    Nfc {
        id: String,
    },
    NfcSecret {
        id: String,
        challenge: String,
        response: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct TokenInput {
    pub amount: i32,
    pub method: AuthenticationInput,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum TokenOutput {
    Authorized {
        token: String,
    },
    AuthenticationNeeded {
        id: String,
        key: String,
        challenge: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct PaymentInput {
    pub token: String,
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

pub fn transaction_token(
    conn: &DbConnection,
    identity: &Identity,
    input: TokenInput,
) -> ServiceResult<TokenOutput> {
    identity.require_cert()?;

    let output = match &input.method {
        AuthenticationInput::Barcode { code } => {
            let account = authentication_barcode::get(conn, &code)?;

            TokenOutput::Authorized {
                token: create_token_from_obj(&Session::create_transaction(
                    conn,
                    &account.id,
                    input.amount,
                )?)?,
            }
        }
        AuthenticationInput::Nfc { id } => {
            let result = authentication_nfc::get(conn, &id)?;
            match result {
                authentication_nfc::NfcResult::Ok { account } => TokenOutput::Authorized {
                    token: create_token_from_obj(&Session::create_transaction(
                        conn,
                        &account.id,
                        input.amount,
                    )?)?,
                },
                authentication_nfc::NfcResult::AuthenticationRequested { key, challenge } => {
                    TokenOutput::AuthenticationNeeded {
                        id: id.clone(),
                        key,
                        challenge,
                    }
                }
                authentication_nfc::NfcResult::WriteKey { .. } => {
                    return Err(ServiceError::Unauthorized);
                }
            }
        }
        AuthenticationInput::NfcSecret {
            id,
            challenge,
            response,
        } => {
            let account =
                authentication_nfc::get_challenge_response(conn, &id, &challenge, &response)?;
            TokenOutput::Authorized {
                token: create_token_from_obj(&Session::create_transaction(
                    conn,
                    &account.id,
                    input.amount,
                )?)?,
            }
        }
    };

    Ok(output)
}

pub fn transaction_payment(
    conn: &DbConnection,
    identity: &Identity,
    input: PaymentInput,
) -> ServiceResult<PaymentOutput> {
    identity.require_cert()?;

    let request_token: Session = parse_obj_from_token(&input.token)?;
    let session = Session::get(conn, &request_token.id)?;
    let transaction_total = if let Some(transaction_total) = session.transaction_total {
        transaction_total
    } else {
        return Err(ServiceError::Unauthorized);
    };
    let mut account = Account::get(conn, &session.account_id)?;

    if input.amount != transaction_total {
        return Err(ServiceError::Unauthorized);
    }

    let transaction = transactions::execute(conn, &mut account, None, input.amount)?;

    let mut products: Vec<(Product, i32)> = Vec::new();

    for (product_id, amount) in &input.products {
        if let Ok(product) = Product::get(conn, &product_id) {
            products.push((product, *amount));
        }
    }

    transaction.add_products(conn, products)?;

    Ok(PaymentOutput {
        account: account.into(),
        transaction: transaction.into(),
    })
}
