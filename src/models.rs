#![allow(unused)]
use std::{collections::HashMap, fmt::Debug, time::Instant};

use chrono::{DateTime, NaiveDate, Utc};

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
    Purchaser,
    Admin,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CardType {
    GenericNfc,
    AsciiMifare,
    HostCardEmulation,
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
    pub depends_on_session: Option<String>,
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
    PasswordResetToken { token: String },
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
            AuthRequest::PasswordResetToken { token } => {
                let mut out = vec![4u8];
                out.extend_from_slice(token.as_bytes());
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
    pub enable_monthly_mail_report: bool,
    pub enable_automatic_stamp_usage: bool,
    pub status: Option<AccountStatus>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AccountStatus {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub priority: u64,
}

#[derive(PartialEq, Clone)]
pub struct Image {
    pub data: Vec<u8>,
    pub mimetype: String,
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field(
                "data",
                &format!("{:?}[..20]", &self.data[..20.min(self.data.len())]),
            )
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

/// One barcode of a product.
///
/// A barcode identifies a *purchasing variant* of the article: the code printed on the single
/// bottle and the code printed on the crate belong to the same product but stand for a different
/// number of single units (`container_size`). Codes that are pure aliases of each other simply
/// use `container_size = 1`.
#[derive(Debug, PartialEq, Clone)]
pub struct ProductBarcode {
    pub id: u64,
    pub code: String,
    /// Single units this code stands for (crate of 20 => 20). Always >= 1.
    pub container_size: i32,
    /// Free text shown next to the code, e.g. "Kasten 20x0,5l".
    pub label: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub price: CoinAmount,
    pub bonus: CoinAmount,
    pub purchase_tax: i32,
    pub nickname: Option<String>,
    pub image: Option<Image>,
    /// All barcodes of the product, oldest first. The first entry is the primary code.
    pub barcodes: Vec<ProductBarcode>,
    pub category: String,
    pub print_lists: Vec<String>,
    pub tags: Vec<String>,
    pub status_prices: Vec<ProductStatusPrice>,
    pub stocked: bool,
    pub target_quantity: i32,
}

impl Product {
    /// The primary barcode of the product (the oldest one), if it has any.
    pub fn primary_barcode(&self) -> Option<&ProductBarcode> {
        self.barcodes.first()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProductStatusPrice {
    pub status: AccountStatus,
    pub price: CoinAmount,
    pub bonus: CoinAmount,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TransactionItem {
    pub effective_price: CoinAmount,
    pub product: Option<Product>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub account: u64,
    pub authorized_by_account_id: Option<u64>,
    pub authorized_with_method: Option<AuthMethodType>,
    pub items: Vec<TransactionItem>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct PaymentItem {
    pub effective_price: CoinAmount,
    pub product_id: Option<u64>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Payment {
    pub account: u64,
    pub items: Vec<PaymentItem>,
    pub authorization: Option<Session>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AuthMethodType {
    PasswordBased,
    NfcBased,
    PublicTab,
    PasswordResetToken,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Session {
    pub account: Account,
    pub token: String,
    pub auth_method: AuthMethodType,
    pub valid_until: DateTime<Utc>,
    pub is_single_use: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RegisterHistory {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub source_register: RegisterHistoryState,
    pub target_register: RegisterHistoryState,
    pub envelope_register: RegisterHistoryState,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RegisterHistoryState {
    pub coin200: i32,
    pub coin100: i32,
    pub coin50: i32,
    pub coin20: i32,
    pub coin10: i32,
    pub coin5: i32,
    pub coin2: i32,
    pub coin1: i32,
    pub note100: i32,
    pub note50: i32,
    pub note20: i32,
    pub note10: i32,
    pub note5: i32,
}

/// Represent a wallet pass
#[derive(Debug, PartialEq, Clone)]
pub struct AppleWalletPass {
    pub account_id: u64,
    pub pass_type_id: String,
    pub authentication_token: String,
    pub qr_code: String,
    pub updated_at: u64,
}

/// Represent a wallet registration
#[derive(Debug, PartialEq, Clone)]
pub struct AppleWalletRegistration {
    pub account_id: u64,
    pub pass_type_id: String,
    pub device_id: String,
    pub push_token: String,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PurchaseState {
    Draft,
    Finalized,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Purchase {
    pub id: u64,
    pub name: String,
    pub store: String,
    pub timestamp: DateTime<Utc>,
    pub state: PurchaseState,
    pub purchased_by_account_id: Option<u64>,
    pub finalized_at: Option<DateTime<Utc>>,
    pub finalized_by_account_id: Option<u64>,
    pub items: Vec<PurchaseItem>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct PurchaseItem {
    pub id: u64,
    pub name: String,
    pub container_size: i32,
    pub container_count: i32,
    pub container_cents: i32,
    pub best_before: Option<NaiveDate>,
    pub barcode: Option<String>,
    pub collected: bool,
    pub product: Option<Product>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum InventoryCheckState {
    Draft,
    Completed,
}

#[derive(Debug, PartialEq, Clone)]
pub struct InventoryCheck {
    pub id: u64,
    pub state: InventoryCheckState,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub started_by_account_id: Option<u64>,
    pub completed_by_account_id: Option<u64>,
    pub generated_purchase_id: Option<u64>,
    pub note: String,
    pub item_count: u64,
    pub counted_count: u64,
    /// Only populated when a single check is loaded, empty for list queries.
    pub items: Vec<InventoryCheckItem>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct InventoryCheckItem {
    pub product: Product,
    pub target_quantity: i32,
    pub counted_quantity: Option<i32>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct InventoryLastPurchase {
    pub purchase_id: u64,
    pub timestamp: DateTime<Utc>,
    pub container_size: i32,
    pub container_cents: i32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct InventoryRow {
    pub product: Product,
    pub target_quantity: i32,
    pub last_counted_quantity: Option<i32>,
    pub last_counted_at: Option<DateTime<Utc>>,
    pub estimated_quantity: Option<i64>,
    pub last_purchase: Option<InventoryLastPurchase>,
}

/// Parameters for completing an inventory check.
#[derive(Debug, PartialEq, Clone)]
pub struct InventoryCheckCompletion {
    pub completed_by_account_id: Option<u64>,
    /// Put every product below its target on the shared shopping list. This is the normal way
    /// a check feeds the next shopping trip: the trip itself is created when someone actually
    /// goes shopping, which is rarely the moment the stock is counted.
    pub add_to_shopping_list: bool,
    /// Prefix of the note on generated shopping list entries; the counted/target numbers are
    /// appended to it.
    pub shopping_list_note: String,
    /// Create a draft purchase for the missing articles right away. Off by default; only useful
    /// when the trip starts immediately after the count.
    pub generate_purchase: bool,
    pub purchase_store: String,
    pub purchase_name: String,
}

/// Filter for shopping list queries.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ShoppingListState {
    Open,
    Done,
    All,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ShoppingListItem {
    pub id: u64,
    pub name: String,
    pub quantity: i32,
    pub note: String,
    pub product: Option<Product>,
    pub created_by_account_id: Option<u64>,
    /// Display name of the creating account (resolved on read, ignored on write).
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub done_at: Option<DateTime<Utc>>,
    pub done_by_account_id: Option<u64>,
    pub purchase_id: Option<u64>,
}
