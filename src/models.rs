#![allow(unused)]
use std::{collections::HashMap, fmt::Debug, time::Instant};

#[derive(Debug, PartialEq, Hash, Eq)]
pub enum CoinType {
    Cent,
    CoffeeStamp,
    BottleStamp,
}

#[derive(Debug, PartialEq)]
pub enum Role {
    Basic,
    Member,
    Admin,
}

#[derive(Debug, PartialEq)]
pub enum CardType {
    NfcId,
    AsciiMifare,
}

#[derive(Debug, PartialEq)]
pub struct AuthPassword {
    pub username: String,
    pub password_hash: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct AuthNfc {
    pub name: String,
    pub card_id: Vec<u8>,
    pub card_type: CardType,
    pub data: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub enum AuthMethod {
    PasswordBased(AuthPassword),
    NfcBased(AuthNfc),
    PublicTab,
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct CoinAmount(pub HashMap<CoinType, i32>);

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
