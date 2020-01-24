use crate::core::{
    authentication_barcode, authentication_nfc, generate_uuid, transactions, Account, DbConnection,
    Pool, Product, ServiceError, ServiceResult, Session, Transaction,
};
use actix_web::{web, HttpResponse};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub id: Uuid,
    pub total: i32,
    pub account_id: Uuid,
}
impl Token {
    pub fn new(conn: &DbConnection, account: &Account, total: i32) -> ServiceResult<Token> {
        let token = Token {
            id: generate_uuid(),
            total,
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
        let _session = Session::get(conn, s)?;

        Self::from_str(s)
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum Authentication {
    Barcode { code: String },
    Nfc { id: String },
    NfcSecret { id: String, secret: String },
}

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub total: i32,
    pub method: Authentication,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum TokenResponse {
    Authorized { token: String },
    AuthenticationNeeded { id: String, key: String },
}

#[derive(Debug, Deserialize)]
pub struct PaymentRequest {
    pub total: i32,
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
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let result = match &token_request.method {
        Authentication::Barcode { code } => {
            let account = authentication_barcode::get(&conn, &code)?;
            TokenResponse::Authorized {
                token: Token::new(&conn, &account, token_request.total)?.to_string()?,
            }
        }
        Authentication::Nfc { id } => {
            let result = authentication_nfc::get_with_secret(&conn, &id, "")?;
            match result {
                authentication_nfc::NfcResult::Ok { account } => TokenResponse::Authorized {
                    token: Token::new(&conn, &account, token_request.total)?.to_string()?,
                },
                authentication_nfc::NfcResult::AuthenticationRequested { key } => {
                    TokenResponse::AuthenticationNeeded {
                        id: id.clone(),
                        key,
                    }
                }
            }
        }
        Authentication::NfcSecret { id, secret } => {
            let result = authentication_nfc::get_with_secret(&conn, &id, &secret)?;
            match result {
                authentication_nfc::NfcResult::Ok { account } => TokenResponse::Authorized {
                    token: Token::new(&conn, &account, token_request.total)?.to_string()?,
                },
                authentication_nfc::NfcResult::AuthenticationRequested { key } => {
                    TokenResponse::AuthenticationNeeded {
                        id: id.clone(),
                        key,
                    }
                }
            }
        }
    };

    Ok(HttpResponse::Ok().json(&result))
}

/// POST route for `/api/v1/transaction/payment`
pub async fn post_transaction_payment(
    pool: web::Data<Pool>,
    payment_request: web::Json<PaymentRequest>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let session = Session::get(&conn, &payment_request.token)?;
    session.delete(&conn)?;

    let token = Token::parse(&conn, &payment_request.token)?;

    let mut account = Account::get(&conn, &token.account_id)?;

    if payment_request.total != token.total {
        return Err(ServiceError::Unauthorized);
    }

    let transaction = transactions::execute(&conn, &mut account, None, payment_request.total)?;

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
