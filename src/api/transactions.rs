use crate::client_cert_required;
use crate::core::{
    authentication_barcode, authentication_nfc, generate_uuid, transactions, Account, DbConnection,
    Pool, Product, ServiceError, ServiceResult, Session, Transaction,
};
use crate::identity_policy::Action;

use actix_web::{web, HttpRequest, HttpResponse};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub id: Uuid,
    pub amount: i32,
    pub account_id: Uuid,
}
impl Token {
    pub fn new(conn: &DbConnection, account: &Account, amount: i32) -> ServiceResult<Token> {
        let token = Token {
            id: generate_uuid(),
            amount,
            account_id: account.id,
        };

        Session::register(conn, &account.id, &token.to_string()?)?;

        Ok(token)
    }

    pub fn to_string(&self) -> ServiceResult<String> {
        let s = serde_json::to_string(&self)?;
        Ok(base64::encode(&s.as_bytes()))
    }

    pub fn from_str(s: &str) -> ServiceResult<Self> {
        let s = base64::decode(s)?;
        let s = String::from_utf8(s)?;
        Ok(serde_json::from_str(&s)?)
    }

    pub fn parse(conn: &DbConnection, s: &str) -> ServiceResult<Self> {
        let session = Session::get(&conn, s)?;
        session.delete(&conn)?;

        Self::from_str(s)
    }
}

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
    pool: web::Data<Pool>,
    token_request: web::Json<TokenRequest>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    client_cert_required!(request, Action::FORBIDDEN);

    let conn = &pool.get()?;

    let result = match &token_request.method {
        Authentication::Barcode { code } => {
            let account = authentication_barcode::get(&conn, &code)?;
            TokenResponse::Authorized {
                token: Token::new(&conn, &account, token_request.amount)?.to_string()?,
            }
        }
        Authentication::Nfc { id } => {
            let result = authentication_nfc::get(&conn, &id)?;
            match result {
                authentication_nfc::NfcResult::Ok { account } => TokenResponse::Authorized {
                    token: Token::new(&conn, &account, token_request.amount)?.to_string()?,
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
                token: Token::new(&conn, &account, token_request.amount)?.to_string()?,
            }
        }
    };

    Ok(HttpResponse::Ok().json(&result))
}

/// POST route for `/api/v1/transaction/payment`
pub async fn post_transaction_payment(
    pool: web::Data<Pool>,
    payment_request: web::Json<PaymentRequest>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    client_cert_required!(request, Action::FORBIDDEN);

    let conn = &pool.get()?;

    let token = Token::parse(&conn, &payment_request.token)?;

    let mut account = Account::get(&conn, &token.account_id)?;

    if payment_request.amount != token.amount {
        return Err(ServiceError::Unauthorized);
    }

    let transaction = transactions::execute(&conn, &mut account, None, payment_request.amount)?;

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
