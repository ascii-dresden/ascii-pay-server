use crate::{
    core::{
        authentication_barcode, authentication_nfc, transactions, Account, Pool, Product,
        ServiceError, ServiceResult, Session, Transaction,
    },
    identity_service::{Identity, IdentityRequire},
    web::utils::{create_token_from_obj, parse_obj_from_token},
};

use actix_web::{web, HttpResponse};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum Authentication {
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
pub struct TokenRequest {
    pub amount: i32,
    pub method: Authentication,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum TokenResponse {
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
pub struct PaymentRequest {
    pub amount: i32,
    pub token: String,
    pub products: HashMap<Uuid, i32>,
}

#[derive(Debug, Serialize)]
pub struct PaymentResponse {
    pub account: Account,
    pub transaction: Transaction,
}

/// POST route for `/api/v1/transaction/token`
pub async fn post_transaction_token(
    identity: Identity,
    pool: web::Data<Pool>,
    token_request: web::Json<TokenRequest>,
) -> ServiceResult<HttpResponse> {
    identity.require_cert()?;
    let conn = &pool.get()?;

    let result = match &token_request.method {
        Authentication::Barcode { code } => {
            let account = authentication_barcode::get(&conn, &code)?;

            TokenResponse::Authorized {
                token: create_token_from_obj(&Session::create_transaction(
                    &conn,
                    &account.id,
                    token_request.amount,
                )?)?,
            }
        }
        Authentication::Nfc { id } => {
            let result = authentication_nfc::get(&conn, &id)?;
            match result {
                authentication_nfc::NfcResult::Ok { account } => TokenResponse::Authorized {
                    token: create_token_from_obj(&Session::create_transaction(
                        &conn,
                        &account.id,
                        token_request.amount,
                    )?)?,
                },
                authentication_nfc::NfcResult::AuthenticationRequested { key, challenge } => {
                    TokenResponse::AuthenticationNeeded {
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
        Authentication::NfcSecret {
            id,
            challenge,
            response,
        } => {
            let account =
                authentication_nfc::get_challenge_response(&conn, &id, &challenge, &response)?;
            TokenResponse::Authorized {
                token: create_token_from_obj(&Session::create_transaction(
                    &conn,
                    &account.id,
                    token_request.amount,
                )?)?,
            }
        }
    };

    Ok(HttpResponse::Ok().json(&result))
}

/// POST route for `/api/v1/transaction/payment`
pub async fn post_transaction_payment(
    identity: Identity,
    pool: web::Data<Pool>,
    payment_request: web::Json<PaymentRequest>,
) -> ServiceResult<HttpResponse> {
    identity.require_cert()?;
    let conn = &pool.get()?;

    let request_token: Session = parse_obj_from_token(&payment_request.token)?;
    let session = Session::get(&conn, &request_token.id)?;
    let transaction_total = if let Some(transaction_total) = session.transaction_total {
        transaction_total
    } else {
        return Err(ServiceError::Unauthorized);
    };
    let mut account = Account::get(&conn, &session.account_id)?;

    if payment_request.amount != transaction_total {
        return Err(ServiceError::Unauthorized);
    }

    let transaction =
        transactions::execute(&conn, &mut account, None, payment_request.amount).await?;

    let mut products: Vec<(Product, i32)> = Vec::new();

    for (product_id, amount) in &payment_request.products {
        if let Ok(product) = Product::get(&conn, &product_id) {
            products.push((product, *amount));
        }
    }

    transaction.add_products(&conn, products)?;

    Ok(HttpResponse::Ok().json(PaymentResponse {
        account,
        transaction,
    }))
}
