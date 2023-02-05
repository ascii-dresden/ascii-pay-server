#![allow(unused)]
use std::{collections::HashMap, fmt::Debug, time::Instant};

#[derive(Debug, PartialEq, Hash, Eq, Clone, Copy)]
pub enum CoinType {
    Cent,
    CoffeeStamp,
    BottleStamp,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Role {
    Basic,
    Member,
    Admin,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CardType {
    NfcId,
    AsciiMifare,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AuthPassword {
    pub username: String,
    pub password_hash: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AuthNfc {
    pub name: String,
    pub card_id: Vec<u8>,
    pub card_type: CardType,
    pub data: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AuthMethod {
    PasswordBased(AuthPassword),
    NfcBased(AuthNfc),
    PublicTab,
}

#[derive(Debug, PartialEq)]
pub enum AuthRequest {
    PasswordBased { username: String },
    NfcBased { card_id: Vec<u8> },
    PublicTab { account_id: u64 },
}

impl AuthRequest {
    pub fn login_key(&self) -> Vec<u8> {
        match self {
            AuthRequest::PasswordBased { username } => {
                let mut out = vec![1u8];
                out.extend_from_slice(username.as_bytes());
                out
            }
            AuthRequest::NfcBased { card_id } => {
                let mut out = vec![2u8];
                out.extend_from_slice(card_id);
                out
            }
            AuthRequest::PublicTab { account_id } => {
                let mut out = vec![3u8];
                out.extend_from_slice(&account_id.to_le_bytes());
                out
            }
        }
    }
}

impl AuthMethod {
    pub fn to_request(&self, account_id: u64) -> AuthRequest {
        match self {
            AuthMethod::PasswordBased(auth) => AuthRequest::PasswordBased {
                username: auth.username.clone(),
            },
            AuthMethod::NfcBased(auth) => AuthRequest::NfcBased {
                card_id: auth.card_id.clone(),
            },
            AuthMethod::PublicTab => AuthRequest::PublicTab { account_id },
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Account {
    pub id: u64,
    pub balance: CoinAmount,
    pub name: String,
    pub email: String,
    pub role: Role,
    pub auth_methods: Vec<AuthMethod>,
}

#[derive(PartialEq)]
pub struct Image {
    pub data: Vec<u8>,
    pub mimetype: String,
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("data", &format!("{:?}[..20]", &self.data[..20]))
            .field("mimetype", &self.mimetype)
            .finish()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CoinAmount(pub HashMap<CoinType, i32>);
impl CoinAmount {
    pub fn zero() -> Self {
        CoinAmount(
            [
                (CoinType::Cent, 0),
                (CoinType::CoffeeStamp, 0),
                (CoinType::BottleStamp, 0),
            ]
            .into_iter()
            .collect(),
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub price: CoinAmount,
    pub bonus: CoinAmount,
    pub nickname: Option<String>,
    pub image: Option<Image>,
    pub barcode: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct TransactionItem {
    pub effective_price: CoinAmount,
    pub product: Option<Product>,
}

#[derive(Debug, PartialEq)]
pub struct Transaction {
    pub id: u64,
    pub timestamp: Instant,
    pub account: u64,
    pub items: Vec<TransactionItem>,
}

#[derive(Debug, PartialEq)]
pub struct PaymentItem {
    pub effective_price: CoinAmount,
    pub product_id: Option<u64>,
}

#[derive(Debug, PartialEq)]
pub struct Payment {
    pub account: u64,
    pub items: Vec<PaymentItem>,
}

#[derive(Debug, PartialEq)]
pub enum AuthMethodType {
    PasswordBased,
    NfcBased,
    PublicTab,
}

#[derive(Debug, PartialEq)]
pub struct Session {
    pub account: Account,
    pub token: String,
    pub auth_method: AuthMethodType,
    pub valid_until: Instant,
    pub is_single_use: bool,
}
