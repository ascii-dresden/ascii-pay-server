#![allow(unused_variables)]

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::types::Json;
use sqlx::{Connection, FromRow, PgPool, Row};
use sqlx::{Pool, Postgres};
use tokio::sync::Mutex;

use crate::error::{ServiceError, ServiceResult};
use crate::models::{
    self, Account, AccountStatus, AppleWalletPass, AppleWalletRegistration, AuthMethod,
    AuthMethodType, AuthNfc, AuthPassword, AuthRequest, CardType, CoinAmount, CoinType, Image,
    PaymentItem, Product, ProductStatusPrice, Role, Session, Transaction, TransactionItem,
};

const MINIMUM_PAYMENT_CENTS: i32 = 0;
const MINIMUM_PAYMENT_BOTTLE_STAMPS: i32 = 0;
const MINIMUM_PAYMENT_COFFEE_STAMPS: i32 = 0;

mod migration;
#[cfg(test)]
mod tests;

pub struct AppStateNfcChallenge {
    pub valid_until: DateTime<Utc>,
    pub rnd_a: Vec<u8>,
    pub rnd_b: Vec<u8>,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<Postgres>,
    pub ascii_mifare_challenge: Arc<Mutex<HashMap<u64, AppStateNfcChallenge>>>,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "tp_account_role", rename_all = "lowercase")]
enum AccountRoleDto {
    Basic,
    Member,
    Purchaser,
    Admin,
}

impl From<AccountRoleDto> for Role {
    fn from(value: AccountRoleDto) -> Self {
        match value {
            AccountRoleDto::Basic => Role::Basic,
            AccountRoleDto::Member => Role::Member,
            AccountRoleDto::Purchaser => Role::Purchaser,
            AccountRoleDto::Admin => Role::Admin,
        }
    }
}

impl From<Role> for AccountRoleDto {
    fn from(value: Role) -> Self {
        match value {
            Role::Basic => AccountRoleDto::Basic,
            Role::Member => AccountRoleDto::Member,
            Role::Purchaser => AccountRoleDto::Purchaser,
            Role::Admin => AccountRoleDto::Admin,
        }
    }
}

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: i64,
    balance_cents: i32,
    balance_coffee_stamps: i32,
    balance_bottle_stamps: i32,
    name: String,
    email: String,
    role: AccountRoleDto,
    auth_methods: Vec<Json<AccountAuthMethodData>>,
    enable_monthly_mail_report: bool,
    enable_automatic_stamp_usage: bool,
    status_id: Option<i64>,
    status_name: Option<String>,
    status_color: Option<String>,
    status_priority: Option<i32>,
}

impl AccountRow {
    fn get_status(self: &AccountRow) -> Option<AccountStatus> {
        let id = self.status_id?;
        let name = self.status_name.as_ref()?;
        let color = self.status_color.as_ref()?;
        let priority = self.status_priority?;

        Some(AccountStatus {
            id: id.try_into().expect("id in database is always positive"),
            name: name.clone(),
            color: color.clone(),
            priority: priority
                .try_into()
                .expect("id in database is always positive"),
        })
    }
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        let status = row.get_status();
        Account {
            id: row
                .id
                .try_into()
                .expect("id in database is always positive"),
            balance: to_coin_amount(&[
                (CoinType::Cent, Some(row.balance_cents)),
                (CoinType::CoffeeStamp, Some(row.balance_coffee_stamps)),
                (CoinType::BottleStamp, Some(row.balance_bottle_stamps)),
            ]),
            name: row.name,
            email: row.email,
            role: row.role.into(),
            auth_methods: row.auth_methods.into_iter().map(|j| j.0.into()).collect(),
            enable_monthly_mail_report: row.enable_monthly_mail_report,
            enable_automatic_stamp_usage: row.enable_automatic_stamp_usage,
            status,
        }
    }
}

#[derive(sqlx::FromRow)]
struct AccountStatusRow {
    id: i64,
    name: String,
    color: String,
    priority: i32,
}

impl From<AccountStatusRow> for AccountStatus {
    fn from(row: AccountStatusRow) -> Self {
        AccountStatus {
            id: row
                .id
                .try_into()
                .expect("id in database is always positive"),
            name: row.name,
            color: row.color,
            priority: row
                .priority
                .try_into()
                .expect("id in database is always positive"),
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct AppleWalletPassRow {
    pub account_id: i64,
    pub pass_type_id: String,
    pub authentication_token: String,
    pub qr_code: String,
    pub updated_at: i64,
}

impl From<AppleWalletPassRow> for AppleWalletPass {
    fn from(row: AppleWalletPassRow) -> Self {
        AppleWalletPass {
            account_id: row
                .account_id
                .try_into()
                .expect("id in database is always positive"),
            pass_type_id: row.pass_type_id,
            authentication_token: row.authentication_token,
            qr_code: row.qr_code,
            updated_at: row
                .updated_at
                .try_into()
                .expect("id in database is always positive"),
        }
    }
}

#[derive(sqlx::FromRow)]
struct AppleWalletRegistrationRow {
    pub account_id: i64,
    pub pass_type_id: String,
    pub device_id: String,
    pub push_token: String,
}

impl From<AppleWalletRegistrationRow> for AppleWalletRegistration {
    fn from(row: AppleWalletRegistrationRow) -> Self {
        AppleWalletRegistration {
            account_id: row
                .account_id
                .try_into()
                .expect("id in database is always positive"),
            pass_type_id: row.pass_type_id,
            device_id: row.device_id,
            push_token: row.push_token,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PaymentAccountRow {
    balance_cents: i32,
    balance_coffee_stamps: i32,
    balance_bottle_stamps: i32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CardTypeDto {
    NfcId,
    AsciiMifare,
    HostCardEmulation,
}

impl From<CardTypeDto> for CardType {
    fn from(value: CardTypeDto) -> Self {
        match value {
            CardTypeDto::NfcId => CardType::GenericNfc,
            CardTypeDto::AsciiMifare => CardType::AsciiMifare,
            CardTypeDto::HostCardEmulation => CardType::HostCardEmulation,
        }
    }
}

impl From<CardType> for CardTypeDto {
    fn from(value: CardType) -> Self {
        match value {
            CardType::GenericNfc => CardTypeDto::NfcId,
            CardType::AsciiMifare => CardTypeDto::AsciiMifare,
            CardType::HostCardEmulation => CardTypeDto::HostCardEmulation,
        }
    }
}

#[derive(Serialize, Deserialize)]
enum AccountAuthMethodData {
    Password {
        username: String,
        password_hash: Vec<u8>,
    },
    Nfc {
        name: String,
        card_id: Vec<u8>,
        card_type: CardTypeDto,
        data: Vec<u8>,
        depends_on_session: Option<String>,
    },
    PublicTab,
}

impl From<AccountAuthMethodData> for AuthMethod {
    fn from(value: AccountAuthMethodData) -> Self {
        match value {
            AccountAuthMethodData::Password {
                username,
                password_hash,
            } => AuthMethod::PasswordBased(AuthPassword {
                username,
                password_hash,
            }),
            AccountAuthMethodData::Nfc {
                name,
                card_id,
                card_type,
                data,
                depends_on_session,
            } => AuthMethod::NfcBased(AuthNfc {
                name,
                card_id,
                card_type: card_type.into(),
                data,
                depends_on_session,
            }),
            AccountAuthMethodData::PublicTab => AuthMethod::PublicTab,
        }
    }
}

impl From<AuthMethod> for AccountAuthMethodData {
    fn from(value: AuthMethod) -> Self {
        match value {
            AuthMethod::PasswordBased(auth) => AccountAuthMethodData::Password {
                username: auth.username,
                password_hash: auth.password_hash,
            },
            AuthMethod::NfcBased(auth) => AccountAuthMethodData::Nfc {
                name: auth.name,
                card_id: auth.card_id,
                card_type: auth.card_type.into(),
                data: auth.data,
                depends_on_session: auth.depends_on_session,
            },
            AuthMethod::PublicTab => AccountAuthMethodData::PublicTab,
        }
    }
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "tp_auth_method_kind", rename_all = "snake_case")]
enum AuthMethodTypeDto {
    Password,
    Nfc,
    PublicTab,
    PasswordResetToken,
}

impl From<AuthMethodTypeDto> for AuthMethodType {
    fn from(value: AuthMethodTypeDto) -> Self {
        match value {
            AuthMethodTypeDto::Password => AuthMethodType::PasswordBased,
            AuthMethodTypeDto::Nfc => AuthMethodType::NfcBased,
            AuthMethodTypeDto::PublicTab => AuthMethodType::PublicTab,
            AuthMethodTypeDto::PasswordResetToken => AuthMethodType::PasswordResetToken,
        }
    }
}

impl From<AuthMethodType> for AuthMethodTypeDto {
    fn from(value: AuthMethodType) -> Self {
        match value {
            AuthMethodType::PasswordBased => AuthMethodTypeDto::Password,
            AuthMethodType::NfcBased => AuthMethodTypeDto::Nfc,
            AuthMethodType::PublicTab => AuthMethodTypeDto::PublicTab,
            AuthMethodType::PasswordResetToken => AuthMethodTypeDto::PasswordResetToken,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    uuid: String,
    #[sqlx(flatten)]
    account: AccountRow,
    auth_method: AuthMethodTypeDto,
    valid_until: DateTime<Utc>,
    is_single_use: bool,
}

impl From<SessionRow> for Session {
    fn from(value: SessionRow) -> Self {
        Session {
            account: value.account.into(),
            token: value.uuid,
            auth_method: value.auth_method.into(),
            valid_until: value.valid_until,
            is_single_use: value.is_single_use,
        }
    }
}

impl AppState {
    pub async fn connect(url: &str) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .expect("connect to database");

        Self::from_pool(pool).await
    }

    pub async fn from_pool(pool: PgPool) -> Self {
        let migrator = Migrator::new(migration::postgresql_migrations())
            .await
            .expect("load migrations");
        migrator.run(&pool).await.expect("run migrations");

        Self {
            pool,
            ascii_mifare_challenge: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

pub struct DatabaseConnection {
    pub connection: sqlx::pool::PoolConnection<sqlx::Postgres>,
}

fn to_service_result<V>(r: Result<V, sqlx::Error>) -> ServiceResult<V> {
    let err = match r {
        Ok(v) => return Ok(v),
        Err(e) => e,
    };

    let bt = std::backtrace::Backtrace::capture();
    error!("SQL query failed: {}\n{}", err, bt);
    Err(ServiceError::InternalServerError(
        "SQL query failed (see server log for details)".to_string(),
    ))
}

fn to_coin_amount(amounts: &[(CoinType, Option<i32>)]) -> CoinAmount {
    CoinAmount(
        amounts
            .iter()
            .filter_map(|(tp, amount)| Some((*tp, (*amount)?)))
            .filter(|v| v.1 != 0)
            .collect(),
    )
}

#[derive(sqlx::FromRow)]
struct ProductImageRow {
    image: Option<Vec<u8>>,
    image_mimetype: Option<String>,
}

impl From<ProductImageRow> for Option<Image> {
    fn from(value: ProductImageRow) -> Self {
        match value.image {
            None => None,
            Some(data) => {
                let mimetype = value
                    .image_mimetype
                    .expect("if image is Some, mimetype must also be Some");
                Some(Image { data, mimetype })
            }
        }
    }
}

#[derive(sqlx::FromRow)]
struct ProductRow {
    id: i64,
    name: String,
    price_cents: i32,
    price_coffee_stamps: i32,
    price_bottle_stamps: i32,
    bonus_cents: i32,
    bonus_coffee_stamps: i32,
    bonus_bottle_stamps: i32,
    purchase_tax: i32,
    nickname: Option<String>,
    #[sqlx(flatten)]
    image: ProductImageRow,
    barcode: Option<String>,
    category: String,
    print_lists: Vec<String>,
    tags: Vec<String>,
    status_id: Vec<i64>,
    status_name: Vec<String>,
    status_color: Vec<String>,
    status_priority: Vec<i32>,
    status_price_cents: Vec<i32>,
    status_price_coffee_stamps: Vec<i32>,
    status_price_bottle_stamps: Vec<i32>,
    status_bonus_cents: Vec<i32>,
    status_bonus_coffee_stamps: Vec<i32>,
    status_bonus_bottle_stamps: Vec<i32>,
}

impl From<ProductRow> for Product {
    fn from(value: ProductRow) -> Self {
        let mut status_price: Vec<ProductStatusPrice> = Vec::new();
        for i in 0..value.status_id.len() {
            let entry = ProductStatusPrice {
                status: AccountStatus {
                    id: value.status_id[i].try_into().expect("IDs are non-negative"),
                    name: value.status_name[i].clone(),
                    color: value.status_color[i].clone(),
                    priority: value.status_priority[i]
                        .try_into()
                        .expect("IDs are non-negative"),
                },
                price: to_coin_amount(&[
                    (CoinType::Cent, Some(value.status_price_cents[i])),
                    (
                        CoinType::BottleStamp,
                        Some(value.status_price_bottle_stamps[i]),
                    ),
                    (
                        CoinType::CoffeeStamp,
                        Some(value.status_price_coffee_stamps[i]),
                    ),
                ]),
                bonus: to_coin_amount(&[
                    (CoinType::Cent, Some(value.status_bonus_cents[i])),
                    (
                        CoinType::BottleStamp,
                        Some(value.status_bonus_bottle_stamps[i]),
                    ),
                    (
                        CoinType::CoffeeStamp,
                        Some(value.status_bonus_coffee_stamps[i]),
                    ),
                ]),
            };
            status_price.push(entry);
        }

        Product {
            id: value.id.try_into().expect("IDs are non-negative"),
            name: value.name,
            price: to_coin_amount(&[
                (CoinType::Cent, Some(value.price_cents)),
                (CoinType::BottleStamp, Some(value.price_bottle_stamps)),
                (CoinType::CoffeeStamp, Some(value.price_coffee_stamps)),
            ]),
            bonus: to_coin_amount(&[
                (CoinType::Cent, Some(value.bonus_cents)),
                (CoinType::BottleStamp, Some(value.bonus_bottle_stamps)),
                (CoinType::CoffeeStamp, Some(value.bonus_coffee_stamps)),
            ]),
            nickname: value.nickname,
            purchase_tax: value.purchase_tax,
            image: value.image.into(),
            barcode: value.barcode,
            category: value.category,
            print_lists: value.print_lists,
            tags: value.tags,
            status_prices: status_price,
        }
    }
}

#[derive(sqlx::FromRow)]
struct TransactionItemRow {
    effective_price_cents: i32,
    effective_price_coffee_stamps: i32,
    effective_price_bottle_stamps: i32,
}

#[derive(sqlx::FromRow)]
struct TransactionRow {
    #[sqlx(try_from = "i64")]
    transaction_id: u64,
    timestamp: DateTime<Utc>,
    account_id: Option<i64>,
    authorized_by_account_id: Option<i64>,
    authorized_with_method: Option<AuthMethodTypeDto>,
    #[sqlx(flatten)]
    item: TransactionItemRow,
}

#[derive(sqlx::FromRow)]
struct RegisterHistoryRow {
    id: i64,
    timestamp: DateTime<Utc>,
    data: Json<RegisterHistoryRowData>,
}

#[derive(Serialize, Deserialize)]
struct RegisterHistoryRowData {
    source_register: RegisterHistoryRowDataState,
    target_register: RegisterHistoryRowDataState,
    envelope_register: RegisterHistoryRowDataState,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct RegisterHistoryRowDataState {
    coin200: i32,
    coin100: i32,
    coin50: i32,
    coin20: i32,
    coin10: i32,
    coin5: i32,
    coin2: i32,
    coin1: i32,
    note100: i32,
    note50: i32,
    note20: i32,
    note10: i32,
    note5: i32,
}

impl From<RegisterHistoryRow> for models::RegisterHistory {
    fn from(value: RegisterHistoryRow) -> Self {
        models::RegisterHistory {
            id: value.id.try_into().expect("IDs are non-negative"),
            timestamp: value.timestamp,
            source_register: value.data.source_register.into(),
            target_register: value.data.target_register.into(),
            envelope_register: value.data.envelope_register.into(),
        }
    }
}

impl From<RegisterHistoryRowDataState> for models::RegisterHistoryState {
    fn from(value: RegisterHistoryRowDataState) -> Self {
        models::RegisterHistoryState {
            coin200: value.coin200,
            coin100: value.coin100,
            coin50: value.coin50,
            coin20: value.coin20,
            coin10: value.coin10,
            coin5: value.coin5,
            coin2: value.coin2,
            coin1: value.coin1,
            note100: value.note100,
            note50: value.note50,
            note20: value.note20,
            note10: value.note10,
            note5: value.note5,
        }
    }
}

impl From<models::RegisterHistoryState> for RegisterHistoryRowDataState {
    fn from(value: models::RegisterHistoryState) -> Self {
        RegisterHistoryRowDataState {
            coin200: value.coin200,
            coin100: value.coin100,
            coin50: value.coin50,
            coin20: value.coin20,
            coin10: value.coin10,
            coin5: value.coin5,
            coin2: value.coin2,
            coin1: value.coin1,
            note100: value.note100,
            note50: value.note50,
            note20: value.note20,
            note10: value.note10,
            note5: value.note5,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PurchaseRow {
    #[sqlx(try_from = "i64")]
    id: u64,
    purchased_by_account_id: Option<i64>,
    store: String,
    timestamp: DateTime<Utc>,
}

impl From<PurchaseRow> for models::Purchase {
    fn from(value: PurchaseRow) -> Self {
        models::Purchase {
            id: value.id,
            timestamp: value.timestamp,
            store: value.store,
            purchased_by_account_id: value
                .purchased_by_account_id
                .map(|id| id.try_into().expect("IDs are non-negative")),
            items: Vec::new(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct PurchaseItemRow {
    #[sqlx(try_from = "i64")]
    purchase_item_id: u64,
    #[sqlx(try_from = "i64")]
    purchase_item_purchase_id: u64,
    purchase_item_name: String,
    purchase_item_container_size: i32,
    purchase_item_container_count: i32,
    purchase_item_container_cents: i32,
}

impl DatabaseConnection {
    pub async fn get_all_accounts(&mut self) -> ServiceResult<Vec<models::Account>> {
        let mut r = sqlx::query_as::<_, AccountRow>(
            r#"
            SELECT
                a.id, a.balance_cents, a.balance_coffee_stamps, a.balance_bottle_stamps,
                a.name, a.email, a.role,
                coalesce(array_agg(account_auth_method.data ORDER BY account_auth_method.id ASC) FILTER (where account_auth_method.id IS NOT NULL), '{}') AS auth_methods,
                a.enable_monthly_mail_report, a.enable_automatic_stamp_usage,
                (array_agg(account_status.id))[1] as status_id,
                (array_agg(account_status.name))[1] as status_name,
                (array_agg(account_status.color))[1] as status_color,
                (array_agg(account_status.priority))[1] as status_priority
            FROM account AS a
                LEFT OUTER JOIN account_auth_method ON a.id = account_auth_method.account_id
                LEFT OUTER JOIN account_status on a.status_id = account_status.id
            GROUP BY a.id
        "#,
        )
        .fetch(self.connection.as_mut());

        let mut out = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;

            out.push(row.into());
        }
        Ok(out)
    }

    pub async fn get_account_by_id(&mut self, id: u64) -> ServiceResult<Option<models::Account>> {
        let r = sqlx::query_as::<_, AccountRow>(
            r#"
            SELECT
                a.id, a.balance_cents, a.balance_coffee_stamps, a.balance_bottle_stamps,
                a.name, a.email, a.role,
                coalesce(array_agg(account_auth_method.data ORDER BY account_auth_method.id ASC) FILTER (where account_auth_method.id IS NOT NULL), '{}') AS auth_methods,
                a.enable_monthly_mail_report, a.enable_automatic_stamp_usage,
                (array_agg(account_status.id))[1] as status_id,
                (array_agg(account_status.name))[1] as status_name,
                (array_agg(account_status.color))[1] as status_color,
                (array_agg(account_status.priority))[1] as status_priority
            FROM account AS a
                LEFT OUTER JOIN account_auth_method ON a.id = account_auth_method.account_id
                LEFT OUTER JOIN account_status on a.status_id = account_status.id
            WHERE a.id = $1
            GROUP BY a.id
        "#)
        .bind(i64::try_from(id).expect("account id is less than 2**63"))
        .fetch_optional(self.connection.as_mut())
        .await;
        let r = to_service_result(r)?;

        Ok(r.map(Account::from))
    }

    pub async fn get_account_by_auth_method(
        &mut self,
        auth_method: AuthRequest,
    ) -> ServiceResult<Option<models::Account>> {
        let r = sqlx::query_as::<_, AccountRow>(
            r#"
            WITH
                matching AS (SELECT account_id FROM account_auth_method WHERE login_key = $1)
            SELECT
                a.id, a.balance_cents, a.balance_coffee_stamps, a.balance_bottle_stamps,
                a.name, a.email, a.role,
                coalesce(array_agg(account_auth_method.data ORDER BY account_auth_method.id ASC) FILTER (where account_auth_method.id IS NOT NULL), '{}') AS auth_methods,
                a.enable_monthly_mail_report, a.enable_automatic_stamp_usage,
                (array_agg(account_status.id))[1] as status_id,
                (array_agg(account_status.name))[1] as status_name,
                (array_agg(account_status.color))[1] as status_color,
                (array_agg(account_status.priority))[1] as status_priority
            FROM account AS a INNER JOIN matching ON matching.account_id = a.id
                LEFT OUTER JOIN account_auth_method ON a.id = account_auth_method.account_id
                LEFT OUTER JOIN account_status on a.status_id = account_status.id
            GROUP BY a.id
        "#)
        .bind(auth_method.login_key())
        .fetch_optional(self.connection.as_mut())
        .await;
        let r = to_service_result(r)?;

        Ok(r.map(Account::from))
    }

    pub async fn create_session_token(
        &mut self,
        account: u64,
        auth_method: models::AuthMethodType,
        valid_until: DateTime<Utc>,
        is_single_use: bool,
    ) -> ServiceResult<String> {
        let r = sqlx::query(
            r#"
            INSERT INTO session (account_id, auth_method, valid_until, is_single_use) VALUES
                ($1, $2, $3, $4)
            RETURNING CAST(uuid AS TEXT)
        "#,
        )
        .bind(i64::try_from(account).expect("account id is less than 2**63"))
        .bind(AuthMethodTypeDto::from(auth_method))
        .bind(valid_until)
        .bind(is_single_use)
        .fetch_one(self.connection.as_mut())
        .await;
        let r = to_service_result(r)?;

        Ok(r.get(0))
    }

    pub async fn delete_session_token(&mut self, session_token: String) -> ServiceResult<()> {
        let r = sqlx::query(
            r#"
            DELETE FROM session WHERE uuid = CAST($1 as UUID)
        "#,
        )
        .bind(session_token)
        .execute(self.connection.as_mut())
        .await;
        to_service_result(r)?;
        Ok(())
    }

    pub async fn cleanup_session_tokens(&mut self) -> ServiceResult<()> {
        let r = sqlx::query(
            r#"
            DELETE FROM session WHERE valid_until < now()
        "#,
        )
        .execute(self.connection.as_mut())
        .await;
        to_service_result(r)?;
        Ok(())
    }

    pub async fn get_session_by_session_token(
        &mut self,
        session_token: String,
    ) -> ServiceResult<Option<models::Session>> {
        let r = sqlx::query_as::<_, SessionRow>(r#"
            WITH
                fulL_account AS (
                    SELECT
                        a.id, a.balance_cents, a.balance_coffee_stamps, a.balance_bottle_stamps,
                        a.name, a.email, a.role,
                        coalesce(array_agg(account_auth_method.data ORDER BY account_auth_method.id ASC) FILTER (where account_auth_method.id IS NOT NULL), '{}') AS auth_methods,
                        a.enable_monthly_mail_report, a.enable_automatic_stamp_usage,
                        (array_agg(account_status.id))[1] as status_id,
                        (array_agg(account_status.name))[1] as status_name,
                        (array_agg(account_status.color))[1] as status_color,
                        (array_agg(account_status.priority))[1] as status_priority
                    FROM account AS a
                        LEFT OUTER JOIN account_auth_method ON a.id = account_auth_method.account_id
                        LEFT OUTER JOIN account_status on a.status_id = account_status.id
                    GROUP BY a.id
                )
            SELECT CAST(session.uuid as TEXT) as uuid, session.auth_method, session.valid_until, session.is_single_use, full_account.*
            FROM full_account INNER JOIN session on full_account.id = session.account_id
            WHERE session.valid_until > now() AND session.uuid = CAST($1 as UUID)
        "#)
        .bind(session_token)
        .fetch_optional(self.connection.as_mut())
        .await;

        if let Ok(r) = r {
            Ok(r.map(Session::from))
        } else {
            Ok(None)
        }
    }

    pub async fn get_sessions_by_account(
        &mut self,
        account_id: u64,
    ) -> ServiceResult<Vec<models::Session>> {
        let account_id = i64::try_from(account_id).expect("ids must fit into i64");
        let mut r = sqlx::query_as::<_, SessionRow>(r#"
            WITH
                fulL_account AS (
                    SELECT
                        a.id, a.balance_cents, a.balance_coffee_stamps, a.balance_bottle_stamps,
                        a.name, a.email, a.role,
                        coalesce(array_agg(account_auth_method.data ORDER BY account_auth_method.id ASC) FILTER (where account_auth_method.id IS NOT NULL), '{}') AS auth_methods,
                        a.enable_monthly_mail_report, a.enable_automatic_stamp_usage,
                        (array_agg(account_status.id))[1] as status_id,
                        (array_agg(account_status.name))[1] as status_name,
                        (array_agg(account_status.color))[1] as status_color,
                        (array_agg(account_status.priority))[1] as status_priority
                    FROM account AS a
                        LEFT OUTER JOIN account_auth_method ON a.id = account_auth_method.account_id
                        LEFT OUTER JOIN account_status on a.status_id = account_status.id
                    GROUP BY a.id
                )
            SELECT CAST(session.uuid as TEXT) as uuid, session.auth_method, session.valid_until, session.is_single_use, full_account.*
            FROM full_account INNER JOIN session on full_account.id = session.account_id
            WHERE session.valid_until > now() AND session.account_id = $1
        "#)
        .bind(account_id)
        .fetch(self.connection.as_mut());

        let mut out = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;

            out.push(row.into());
        }
        Ok(out)
    }

    pub async fn store_account(
        &mut self,
        mut account: models::Account,
    ) -> ServiceResult<models::Account> {
        let q = if account.id == 0 {
            sqlx::query(
                r#"
                INSERT INTO account (balance_cents, balance_coffee_stamps, balance_bottle_stamps, name, email, role, enable_monthly_mail_report, enable_automatic_stamp_usage, status_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING id
            "#,
            )
        } else {
            sqlx::query(r#"
                WITH
                    delete AS (DELETE FROM account_auth_method WHERE account_id = $1)
                UPDATE account
                SET balance_cents = $2, balance_coffee_stamps = $3, balance_bottle_stamps = $4, name = $5, email = $6, role = $7, enable_monthly_mail_report = $8, enable_automatic_stamp_usage = $9, status_id = $10
                WHERE id = $1
                RETURNING id

            "#).bind(i64::try_from(account.id).expect("account id is less than 2**63"))
        };
        let r = q
            .bind(account.balance.0.get(&CoinType::Cent).unwrap_or(&0))
            .bind(account.balance.0.get(&CoinType::CoffeeStamp).unwrap_or(&0))
            .bind(account.balance.0.get(&CoinType::BottleStamp).unwrap_or(&0))
            .bind(&account.name)
            .bind(&account.email)
            .bind(AccountRoleDto::from(account.role))
            .bind(account.enable_monthly_mail_report)
            .bind(account.enable_automatic_stamp_usage)
            .bind(
                account
                    .status
                    .as_ref()
                    .map(|s| i64::try_from(s.id).expect("status id is less than 2**63")),
            )
            .fetch_one(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;
        let account_id = r.get::<i64, _>(0);
        account.id = account_id.try_into().expect("id is always positive");

        let r = sqlx::query(
            r#"
            INSERT INTO account_auth_method (account_id, login_key, data, depends_on_session)
            SELECT $1, login_key, data, CAST(depends_on_session AS UUID) as depends_on_session
            FROM UNNEST($2, $3, $4) AS input (login_key, data, depends_on_session)
        "#,
        )
        .bind(account_id)
        .bind(
            account
                .auth_methods
                .iter()
                .map(|m| m.to_request(account.id).login_key())
                .collect::<Vec<_>>(),
        )
        .bind(
            account
                .auth_methods
                .iter()
                .map(|m| {
                    serde_json::to_value(AccountAuthMethodData::from(m.clone()))
                        .expect("to json cannot fail")
                })
                .collect::<Vec<_>>(),
        )
        .bind(
            account
                .auth_methods
                .iter()
                .map(|m| {
                    if let AuthMethod::NfcBased(ref nfc_based) = m {
                        nfc_based.depends_on_session.clone()
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
        )
        .execute(self.connection.as_mut())
        .await;
        to_service_result(r)?;

        Ok(account)
    }

    pub async fn delete_account(&mut self, id: u64) -> ServiceResult<()> {
        let r = sqlx::query(
            r#"
            DELETE FROM account WHERE id = $1
        "#,
        )
        .bind(i64::try_from(id).expect("account id is less than 2**63"))
        .execute(self.connection.as_mut())
        .await;
        to_service_result(r)?;
        Ok(())
    }

    pub async fn get_all_account_status(&mut self) -> ServiceResult<Vec<models::AccountStatus>> {
        let mut r = sqlx::query_as::<_, AccountStatusRow>(
            r#"
            SELECT
                id, name, priority, color
            FROM account_status
            "#,
        )
        .fetch(self.connection.as_mut());

        let mut out = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;
            out.push(row.into());
        }

        Ok(out)
    }

    pub async fn get_account_status_by_id(
        &mut self,
        id: u64,
    ) -> ServiceResult<Option<models::AccountStatus>> {
        let r = sqlx::query_as::<_, AccountStatusRow>(
            r#"
            SELECT
                id, name, color, priority
            FROM account_status
            WHERE
                id = $1
            "#,
        )
        .bind(i64::try_from(id).expect("ids are less than 2**63"))
        .fetch_optional(self.connection.as_mut())
        .await;

        Ok(to_service_result(r)?.map(AccountStatus::from))
    }

    pub async fn store_account_status(
        &mut self,
        mut account_status: models::AccountStatus,
    ) -> ServiceResult<models::AccountStatus> {
        let q = if account_status.id == 0 {
            sqlx::query(
                r#"
            INSERT INTO account_status (
                name,
                color,
                priority
            ) VALUES (
                $1,
                $2,
                $3
            ) RETURNING id
            "#,
            )
        } else {
            sqlx::query(
                r#"
                UPDATE account_status
                SET
                    name = $2,
                    color = $3,
                    priority = $4
                WHERE id = $1
                RETURNING id
            "#,
            )
            .bind(i64::try_from(account_status.id).expect("product id is less than 2**63"))
        };
        let r = q
            .bind(&account_status.name)
            .bind(&account_status.color)
            .bind(i32::try_from(account_status.priority).expect("product id is less than 2**31"))
            .fetch_one(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;
        account_status.id = r
            .get::<i64, _>(0)
            .try_into()
            .expect("id is always positive");

        Ok(account_status)
    }

    pub async fn delete_account_status(&mut self, id: u64) -> ServiceResult<()> {
        let id = i64::try_from(id).expect("id is always less than 2**63");
        let r = sqlx::query(r#"DELETE FROM account_status WHERE id = $1"#)
            .bind(id)
            .execute(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;
        if r.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    pub async fn get_all_products(&mut self) -> ServiceResult<Vec<models::Product>> {
        let mut r = sqlx::query_as::<_, ProductRow>(
            r#"
            SELECT
                p.id, p.name,
                p.price_cents, p.price_coffee_stamps, p.price_bottle_stamps,
                p.bonus_cents, p.bonus_coffee_stamps, p.bonus_bottle_stamps,
                p.nickname, p.purchase_tax,
                NULL AS image, NULL AS image_mimetype,
                p.barcode, p.category, p.print_lists, p.tags,
                coalesce(array_agg(account_status.id) FILTER (where account_status.id IS NOT NULL), '{}') as status_id,
                coalesce(array_agg(account_status.name) FILTER (where account_status.name IS NOT NULL), '{}') as status_name,
                coalesce(array_agg(account_status.color) FILTER (where account_status.color IS NOT NULL), '{}') as status_color,
                coalesce(array_agg(account_status.priority) FILTER (where account_status.priority IS NOT NULL), '{}') as status_priority,
                coalesce(array_agg(product_status_price.price_cents) FILTER (where product_status_price.price_cents IS NOT NULL), '{}') as status_price_cents,
                coalesce(array_agg(product_status_price.price_bottle_stamps) FILTER (where product_status_price.price_bottle_stamps IS NOT NULL), '{}') as status_price_bottle_stamps,
                coalesce(array_agg(product_status_price.price_coffee_stamps) FILTER (where product_status_price.price_coffee_stamps IS NOT NULL), '{}') as status_price_coffee_stamps,
                coalesce(array_agg(product_status_price.bonus_cents) FILTER (where product_status_price.bonus_cents IS NOT NULL), '{}') as status_bonus_cents,
                coalesce(array_agg(product_status_price.bonus_bottle_stamps) FILTER (where product_status_price.bonus_bottle_stamps IS NOT NULL), '{}') as status_bonus_bottle_stamps,
                coalesce(array_agg(product_status_price.bonus_coffee_stamps) FILTER (where product_status_price.bonus_coffee_stamps IS NOT NULL), '{}') as status_bonus_coffee_stamps
            FROM product AS p
                    LEFT OUTER JOIN product_status_price ON p.id = product_status_price.product_id
                    LEFT OUTER JOIN account_status on product_status_price.status_id = account_status.id
            GROUP BY p.id
            "#,
        )
        .fetch(self.connection.as_mut());

        let mut out = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;
            out.push(row.into());
        }

        Ok(out)
    }

    pub async fn get_product_by_id(&mut self, id: u64) -> ServiceResult<Option<models::Product>> {
        let r = sqlx::query_as::<_, ProductRow>(
            r#"
            SELECT
                p.id, p.name,
                p.price_cents, p.price_coffee_stamps, p.price_bottle_stamps,
                p.bonus_cents, p.bonus_coffee_stamps, p.bonus_bottle_stamps,
                p.nickname, p.purchase_tax,
                NULL AS image, NULL AS image_mimetype,
                p.barcode, p.category, p.print_lists, p.tags,
                coalesce(array_agg(account_status.id) FILTER (where account_status.id IS NOT NULL), '{}') as status_id,
                coalesce(array_agg(account_status.name) FILTER (where account_status.name IS NOT NULL), '{}') as status_name,
                coalesce(array_agg(account_status.color) FILTER (where account_status.color IS NOT NULL), '{}') as status_color,
                coalesce(array_agg(account_status.priority) FILTER (where account_status.priority IS NOT NULL), '{}') as status_priority,
                coalesce(array_agg(product_status_price.price_cents) FILTER (where product_status_price.price_cents IS NOT NULL), '{}') as status_price_cents,
                coalesce(array_agg(product_status_price.price_bottle_stamps) FILTER (where product_status_price.price_bottle_stamps IS NOT NULL), '{}') as status_price_bottle_stamps,
                coalesce(array_agg(product_status_price.price_coffee_stamps) FILTER (where product_status_price.price_coffee_stamps IS NOT NULL), '{}') as status_price_coffee_stamps,
                coalesce(array_agg(product_status_price.bonus_cents) FILTER (where product_status_price.bonus_cents IS NOT NULL), '{}') as status_bonus_cents,
                coalesce(array_agg(product_status_price.bonus_bottle_stamps) FILTER (where product_status_price.bonus_bottle_stamps IS NOT NULL), '{}') as status_bonus_bottle_stamps,
                coalesce(array_agg(product_status_price.bonus_coffee_stamps) FILTER (where product_status_price.bonus_coffee_stamps IS NOT NULL), '{}') as status_bonus_coffee_stamps
            FROM product AS p
                    LEFT OUTER JOIN product_status_price ON p.id = product_status_price.product_id
                    LEFT OUTER JOIN account_status on product_status_price.status_id = account_status.id
            WHERE
                p.id = $1
            GROUP BY p.id
            "#,
        )
        .bind(i64::try_from(id).expect("ids are less than 2**63"))
        .fetch_optional(self.connection.as_mut())
        .await;

        Ok(to_service_result(r)?.map(Product::from))
    }

    pub async fn store_product(
        &mut self,
        mut product: models::Product,
    ) -> ServiceResult<models::Product> {
        let q = if product.id == 0 {
            sqlx::query(
                r#"
            INSERT INTO product (
                name,
                price_cents,
                price_coffee_stamps,
                price_bottle_stamps,
                bonus_cents,
                bonus_coffee_stamps,
                bonus_bottle_stamps,
                nickname,
                purchase_tax,
                barcode,
                category,
                print_lists,
                tags
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                $10,
                $11,
                $12,
                $13
            ) RETURNING id
            "#,
            )
        } else {
            sqlx::query(
                r#"
                WITH
                    delete AS (DELETE FROM product_status_price WHERE product_id = $1)
                UPDATE product
                SET
                    name = $2,
                    price_cents = $3,
                    price_coffee_stamps = $4,
                    price_bottle_stamps = $5,
                    bonus_cents = $6,
                    bonus_coffee_stamps = $7,
                    bonus_bottle_stamps = $8,
                    nickname = $9,
                    purchase_tax = $10,
                    barcode = $11,
                    category = $12,
                    print_lists = $13,
                    tags = $14
                WHERE id = $1
                RETURNING id
            "#,
            )
            .bind(i64::try_from(product.id).expect("product id is less than 2**63"))
        };
        let r = q
            .bind(&product.name)
            .bind(product.price.0.get(&CoinType::Cent).unwrap_or(&0))
            .bind(product.price.0.get(&CoinType::CoffeeStamp).unwrap_or(&0))
            .bind(product.price.0.get(&CoinType::BottleStamp).unwrap_or(&0))
            .bind(product.bonus.0.get(&CoinType::Cent).unwrap_or(&0))
            .bind(product.bonus.0.get(&CoinType::CoffeeStamp).unwrap_or(&0))
            .bind(product.bonus.0.get(&CoinType::BottleStamp).unwrap_or(&0))
            .bind(&product.nickname)
            .bind(product.purchase_tax)
            .bind(&product.barcode)
            .bind(&product.category)
            .bind(&product.print_lists)
            .bind(&product.tags)
            .fetch_one(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;

        let product_id = r.get::<i64, _>(0);
        product.id = product_id.try_into().expect("id is always positive");

        let r = sqlx::query(
            r#"
            INSERT INTO product_status_price (product_id, status_id, price_cents, price_coffee_stamps, price_bottle_stamps, bonus_cents, bonus_coffee_stamps, bonus_bottle_stamps)
            SELECT $1, status_id, price_cents, price_coffee_stamps, price_bottle_stamps, bonus_cents, bonus_coffee_stamps, bonus_bottle_stamps
            FROM UNNEST($2, $3, $4, $5, $6, $7, $8) AS input (status_id, price_cents, price_coffee_stamps, price_bottle_stamps, bonus_cents, bonus_coffee_stamps, bonus_bottle_stamps)
        "#,
        )
        .bind(product_id)
        .bind(
            product
                .status_prices
                .iter()
                .map(|p| i64::try_from(p.status.id).expect("product id is less than 2**63"))
                .collect::<Vec<_>>(),
        )
        .bind(
            product
                .status_prices
                .iter()
                .map(|p| *p.price.0.get(&CoinType::Cent).unwrap_or(&0))
                .collect::<Vec<_>>(),
        )
        .bind(
            product
                .status_prices
                .iter()
                .map(|p| *p.price.0.get(&CoinType::CoffeeStamp).unwrap_or(&0))
                .collect::<Vec<_>>(),
        )
        .bind(
            product
                .status_prices
                .iter()
                .map(|p| *p.price.0.get(&CoinType::BottleStamp).unwrap_or(&0))
                .collect::<Vec<_>>(),
        )
        .bind(
            product
                .status_prices
                .iter()
                .map(|p| *p.bonus.0.get(&CoinType::Cent).unwrap_or(&0))
                .collect::<Vec<_>>(),
        )
        .bind(
            product
                .status_prices
                .iter()
                .map(|p| *p.bonus.0.get(&CoinType::CoffeeStamp).unwrap_or(&0))
                .collect::<Vec<_>>(),
        )
        .bind(
            product
                .status_prices
                .iter()
                .map(|p| *p.bonus.0.get(&CoinType::BottleStamp).unwrap_or(&0))
                .collect::<Vec<_>>(),
        )
        .execute(self.connection.as_mut())
        .await;
        to_service_result(r)?;

        Ok(product)
    }

    pub async fn delete_product(&mut self, id: u64) -> ServiceResult<()> {
        let id = i64::try_from(id).expect("id is always less than 2**63");
        let r = sqlx::query(r#"DELETE FROM product WHERE id = $1"#)
            .bind(id)
            .execute(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;
        if r.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    pub async fn get_product_image(&mut self, id: u64) -> ServiceResult<Option<models::Image>> {
        let id = i64::try_from(id).expect("id is always less than 2**63");
        let r = sqlx::query_as::<_, ProductImageRow>(
            r#"SELECT image, image_mimetype FROM product WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(self.connection.as_mut())
        .await;
        let r = to_service_result(r)?;
        Ok(r.and_then(From::from))
    }

    pub async fn store_product_image(
        &mut self,
        id: u64,
        image: models::Image,
    ) -> ServiceResult<()> {
        let id = i64::try_from(id).expect("id is always less than 2**63");
        let r = sqlx::query(r#"UPDATE product SET image = $2, image_mimetype = $3 WHERE id = $1"#)
            .bind(id)
            .bind(image.data)
            .bind(image.mimetype)
            .execute(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;
        if r.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    pub async fn delete_product_image(&mut self, id: u64) -> ServiceResult<()> {
        let id = i64::try_from(id).expect("id is always less than 2**63");
        let r =
            sqlx::query(r#"UPDATE product SET image = NULL, image_mimetype = NULL WHERE id = $1"#)
                .bind(id)
                .execute(self.connection.as_mut())
                .await;
        let r = to_service_result(r)?;
        if r.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    pub async fn get_transactions(&mut self) -> ServiceResult<Vec<models::Transaction>> {
        let mut r = sqlx::query(
            r#"
                SELECT
                    item.transaction_id as transaction_id,
                    item.timestamp as timestamp,
                    item.account_id as account_id,
                    item.effective_price_cents as effective_price_cents,
                    item.effective_price_coffee_stamps as effective_price_coffee_stamps,
                    item.effective_price_bottle_stamps as effective_price_bottle_stamps,
                    item.authorized_by_account_id as authorized_by_account_id,
                    item.authorized_with_method as authorized_with_method,
                    p.id as id,
                    p.name as name,
                    p.price_cents as price_cents,
                    p.price_coffee_stamps as price_coffee_stamps,
                    p.price_bottle_stamps as price_bottle_stamps,
                    p.bonus_cents as bonus_cents,
                    p.bonus_coffee_stamps as bonus_coffee_stamps,
                    p.bonus_bottle_stamps as bonus_bottle_stamps,
                    p.nickname as nickname,
                    p.purchase_tax as purchase_tax,
                    NULL as image,
                    NULL as image_mimetype,
                    p.barcode as barcode,
                    p.category as category,
                    p.print_lists as print_lists,
                    p.tags as tags,
                    coalesce(array_agg(account_status.id) FILTER (where account_status.id IS NOT NULL), '{}') as status_id,
                    coalesce(array_agg(account_status.name) FILTER (where account_status.name IS NOT NULL), '{}') as status_name,
                    coalesce(array_agg(account_status.color) FILTER (where account_status.color IS NOT NULL), '{}') as status_color,
                    coalesce(array_agg(account_status.priority) FILTER (where account_status.priority IS NOT NULL), '{}') as status_priority,
                    coalesce(array_agg(product_status_price.price_cents) FILTER (where product_status_price.price_cents IS NOT NULL), '{}') as status_price_cents,
                    coalesce(array_agg(product_status_price.price_bottle_stamps) FILTER (where product_status_price.price_bottle_stamps IS NOT NULL), '{}') as status_price_bottle_stamps,
                    coalesce(array_agg(product_status_price.price_coffee_stamps) FILTER (where product_status_price.price_coffee_stamps IS NOT NULL), '{}') as status_price_coffee_stamps,
                    coalesce(array_agg(product_status_price.bonus_cents) FILTER (where product_status_price.bonus_cents IS NOT NULL), '{}') as status_bonus_cents,
                    coalesce(array_agg(product_status_price.bonus_bottle_stamps) FILTER (where product_status_price.bonus_bottle_stamps IS NOT NULL), '{}') as status_bonus_bottle_stamps,
                    coalesce(array_agg(product_status_price.bonus_coffee_stamps) FILTER (where product_status_price.bonus_coffee_stamps IS NOT NULL), '{}') as status_bonus_coffee_stamps
                FROM
                    transaction_item item
                        LEFT OUTER JOIN product p ON item.product_id = p.id
                        LEFT OUTER JOIN product_status_price ON p.id = product_status_price.product_id
                        LEFT OUTER JOIN account_status on product_status_price.status_id = account_status.id
                GROUP BY item.id, p.id
                ORDER BY item.timestamp ASC, item.transaction_id ASC
            "#,
        )
        .fetch(self.connection.as_mut());

        let mut out: Vec<Transaction> = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;

            let new_tx = extend_transaction_with_row(out.last_mut(), row)?;
            if let Some(tx) = new_tx {
                out.push(tx);
            }
        }

        Ok(out)
    }

    pub async fn get_transactions_by_account(
        &mut self,
        account_id: u64,
    ) -> ServiceResult<Vec<models::Transaction>> {
        let id = i64::try_from(account_id).expect("id is always less than 2**63");
        let mut r = sqlx::query(
            r#"
                SELECT
                    item.transaction_id as transaction_id,
                    item.timestamp as timestamp,
                    item.account_id as account_id,
                    item.effective_price_cents as effective_price_cents,
                    item.effective_price_coffee_stamps as effective_price_coffee_stamps,
                    item.effective_price_bottle_stamps as effective_price_bottle_stamps,
                    item.authorized_by_account_id as authorized_by_account_id,
                    item.authorized_with_method as authorized_with_method,
                    p.id as id,
                    p.name as name,
                    p.price_cents as price_cents,
                    p.price_coffee_stamps as price_coffee_stamps,
                    p.price_bottle_stamps as price_bottle_stamps,
                    p.bonus_cents as bonus_cents,
                    p.bonus_coffee_stamps as bonus_coffee_stamps,
                    p.bonus_bottle_stamps as bonus_bottle_stamps,
                    p.nickname as nickname,
                    p.purchase_tax as purchase_tax,
                    NULL as image,
                    NULL as image_mimetype,
                    p.barcode as barcode,
                    p.category as category,
                    p.print_lists as print_lists,
                    p.tags as tags,
                    coalesce(array_agg(account_status.id) FILTER (where account_status.id IS NOT NULL), '{}') as status_id,
                    coalesce(array_agg(account_status.name) FILTER (where account_status.name IS NOT NULL), '{}') as status_name,
                    coalesce(array_agg(account_status.color) FILTER (where account_status.color IS NOT NULL), '{}') as status_color,
                    coalesce(array_agg(account_status.priority) FILTER (where account_status.priority IS NOT NULL), '{}') as status_priority,
                    coalesce(array_agg(product_status_price.price_cents) FILTER (where product_status_price.price_cents IS NOT NULL), '{}') as status_price_cents,
                    coalesce(array_agg(product_status_price.price_bottle_stamps) FILTER (where product_status_price.price_bottle_stamps IS NOT NULL), '{}') as status_price_bottle_stamps,
                    coalesce(array_agg(product_status_price.price_coffee_stamps) FILTER (where product_status_price.price_coffee_stamps IS NOT NULL), '{}') as status_price_coffee_stamps,
                    coalesce(array_agg(product_status_price.bonus_cents) FILTER (where product_status_price.bonus_cents IS NOT NULL), '{}') as status_bonus_cents,
                    coalesce(array_agg(product_status_price.bonus_bottle_stamps) FILTER (where product_status_price.bonus_bottle_stamps IS NOT NULL), '{}') as status_bonus_bottle_stamps,
                    coalesce(array_agg(product_status_price.bonus_coffee_stamps) FILTER (where product_status_price.bonus_coffee_stamps IS NOT NULL), '{}') as status_bonus_coffee_stamps
                FROM
                    transaction_item item
                        LEFT OUTER JOIN product p ON item.product_id = p.id
                        LEFT OUTER JOIN product_status_price ON p.id = product_status_price.product_id
                        LEFT OUTER JOIN account_status on product_status_price.status_id = account_status.id
                WHERE
                    (item.account_id = $1) OR ($1 = 0 AND item.account_id IS NULL)
                GROUP BY item.id, p.id
                ORDER BY item.timestamp ASC, item.transaction_id ASC
            "#,
        )
        .bind(id)
        .fetch(self.connection.as_mut());

        let mut out: Vec<Transaction> = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;

            let new_tx = extend_transaction_with_row(out.last_mut(), row)?;
            if let Some(tx) = new_tx {
                out.push(tx);
            }
        }

        Ok(out)
    }

    pub async fn get_transaction_by_id(
        &mut self,
        id: u64,
    ) -> ServiceResult<Option<models::Transaction>> {
        let id = i64::try_from(id).expect("id is always less than 2**63");
        let mut r = sqlx::query(
            r#"
            SELECT
                item.transaction_id as transaction_id,
                item.timestamp as timestamp,
                item.account_id as account_id,
                item.effective_price_cents as effective_price_cents,
                item.effective_price_coffee_stamps as effective_price_coffee_stamps,
                item.effective_price_bottle_stamps as effective_price_bottle_stamps,
                item.authorized_by_account_id as authorized_by_account_id,
                item.authorized_with_method as authorized_with_method,
                p.id as id,
                p.name as name,
                p.price_cents as price_cents,
                p.price_coffee_stamps as price_coffee_stamps,
                p.price_bottle_stamps as price_bottle_stamps,
                p.bonus_cents as bonus_cents,
                p.bonus_coffee_stamps as bonus_coffee_stamps,
                p.bonus_bottle_stamps as bonus_bottle_stamps,
                p.nickname as nickname,
                p.purchase_tax as purchase_tax,
                NULL as image,
                NULL as image_mimetype,
                p.barcode as barcode,
                p.category as category,
                p.print_lists as print_lists,
                p.tags as tags,
                coalesce(array_agg(account_status.id) FILTER (where account_status.id IS NOT NULL), '{}') as status_id,
                coalesce(array_agg(account_status.name) FILTER (where account_status.name IS NOT NULL), '{}') as status_name,
                coalesce(array_agg(account_status.color) FILTER (where account_status.color IS NOT NULL), '{}') as status_color,
                coalesce(array_agg(account_status.priority) FILTER (where account_status.priority IS NOT NULL), '{}') as status_priority,
                coalesce(array_agg(product_status_price.price_cents) FILTER (where product_status_price.price_cents IS NOT NULL), '{}') as status_price_cents,
                coalesce(array_agg(product_status_price.price_bottle_stamps) FILTER (where product_status_price.price_bottle_stamps IS NOT NULL), '{}') as status_price_bottle_stamps,
                coalesce(array_agg(product_status_price.price_coffee_stamps) FILTER (where product_status_price.price_coffee_stamps IS NOT NULL), '{}') as status_price_coffee_stamps,
                coalesce(array_agg(product_status_price.bonus_cents) FILTER (where product_status_price.bonus_cents IS NOT NULL), '{}') as status_bonus_cents,
                coalesce(array_agg(product_status_price.bonus_bottle_stamps) FILTER (where product_status_price.bonus_bottle_stamps IS NOT NULL), '{}') as status_bonus_bottle_stamps,
                coalesce(array_agg(product_status_price.bonus_coffee_stamps) FILTER (where product_status_price.bonus_coffee_stamps IS NOT NULL), '{}') as status_bonus_coffee_stamps
            FROM
                transaction_item item
                    LEFT OUTER JOIN product p ON item.product_id = p.id
                    LEFT OUTER JOIN product_status_price ON p.id = product_status_price.product_id
                    LEFT OUTER JOIN account_status on product_status_price.status_id = account_status.id
            WHERE item.transaction_id = $1
            GROUP BY item.id, p.id
            "#,
        )
        .bind(id)
        .fetch(self.connection.as_mut());

        let mut tx = None;
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;
            let new_tx = extend_transaction_with_row(tx.as_mut(), row)?;
            if new_tx.is_some() {
                assert!(tx.is_none(), "there should be at most one tx");
                tx = new_tx;
            }
        }

        Ok(tx)
    }

    pub async fn payment(
        &mut self,
        payment: models::Payment,
        timestamp: DateTime<Utc>,
        check_payment_conditions: bool,
    ) -> ServiceResult<models::Transaction> {
        fn get_type_amounts(t: CoinType, items: &[PaymentItem]) -> Vec<i32> {
            items
                .iter()
                .map(|item| *item.effective_price.0.get(&t).unwrap_or(&0))
                .collect()
        }

        let total_price_cents: i32 = get_type_amounts(CoinType::Cent, &payment.items)
            .into_iter()
            .sum();
        let total_price_bottle_stamps: i32 =
            get_type_amounts(CoinType::BottleStamp, &payment.items)
                .into_iter()
                .sum();
        let total_price_coffee_stamps: i32 =
            get_type_amounts(CoinType::CoffeeStamp, &payment.items)
                .into_iter()
                .sum();

        let mut transaction = self.connection.begin().await?;

        if check_payment_conditions {
            let allow_credit_loading = if let Some(ref session) = payment.authorization {
                matches!(session.account.role, Role::Admin)
                    || matches!(session.auth_method, AuthMethodType::NfcBased)
            } else {
                true
            };

            if !allow_credit_loading {
                for item in payment.items.iter() {
                    if item
                        .effective_price
                        .0
                        .get(&CoinType::Cent)
                        .copied()
                        .unwrap_or(0)
                        < 0
                    {
                        return ServiceResult::Err(ServiceError::Forbidden);
                    }
                }
            }

            let r = sqlx::query_as::<_, PaymentAccountRow>(
                r#"
            SELECT a.balance_cents, a.balance_coffee_stamps, a.balance_bottle_stamps
            FROM account AS a
            WHERE a.id = $1
        "#,
            )
            .bind(i64::try_from(payment.account).expect("account id is less than 2**63"))
            .fetch_optional(transaction.as_mut())
            .await;

            match to_service_result(r)? {
                Some(account) => {
                    let mut errors: Vec<String> = Vec::new();

                    let new_balance_cents = account.balance_cents - total_price_cents;
                    if new_balance_cents < MINIMUM_PAYMENT_CENTS
                        && new_balance_cents < account.balance_cents
                    {
                        errors.push(String::from("Cent"));
                    }

                    let new_balance_bottle_stamps =
                        account.balance_bottle_stamps - total_price_bottle_stamps;
                    if new_balance_bottle_stamps < MINIMUM_PAYMENT_BOTTLE_STAMPS
                        && new_balance_bottle_stamps < account.balance_bottle_stamps
                    {
                        errors.push(String::from("BottleStamp"));
                    }

                    let new_balance_coffee_stamps =
                        account.balance_coffee_stamps - total_price_coffee_stamps;
                    if new_balance_coffee_stamps < MINIMUM_PAYMENT_COFFEE_STAMPS
                        && new_balance_coffee_stamps < account.balance_coffee_stamps
                    {
                        errors.push(String::from("CoffeeStamp"));
                    }

                    if !errors.is_empty() {
                        return ServiceResult::Err(ServiceError::PaymentError(errors));
                    }
                }
                None => {
                    return ServiceResult::Err(ServiceError::NotFound);
                }
            }
        }

        let r = sqlx::query(
            r#"
            WITH
                transaction_args AS (
                    SELECT
                        nextval('transaction_id_seq') AS transaction_id,
                        $1 AS timestamp,
                        $2 AS account_id,
                        $10 AS authorized_by_account_id,
                        $11 AS authorized_with_method
                ),
                updated AS (
                    UPDATE account
                    SET
                        balance_cents = balance_cents - $7,
                        balance_bottle_stamps = balance_bottle_stamps - $8,
                        balance_coffee_stamps = balance_coffee_stamps - $9
                    WHERE
                        id = $2
                ),
                inserted AS (
                    INSERT INTO transaction_item (
                        transaction_id,
                        effective_price_cents,
                        effective_price_bottle_stamps,
                        effective_price_coffee_stamps,
                        product_id,
                        timestamp,
                        account_id,
                        authorized_by_account_id,
                        authorized_with_method
                    )
                    SELECT
                        transaction_id,
                        effective_price_cents,
                        effective_price_bottle_stamps,
                        effective_price_coffee_stamps,
                        product_id,
                        timestamp,
                        account_id,
                        authorized_by_account_id,
                        authorized_with_method
                    FROM
                        transaction_args,
                        UNNEST($3, $4, $5, $6) AS item_args(
                            effective_price_cents,
                            effective_price_bottle_stamps,
                            effective_price_coffee_stamps,
                            product_id
                        )
                    RETURNING
                        id AS transaction_item_id,
                        transaction_id,
                        effective_price_cents,
                        effective_price_bottle_stamps,
                        effective_price_coffee_stamps,
                        product_id,
                        timestamp,
                        account_id,
                        authorized_by_account_id,
                        authorized_with_method
                    )
            SELECT
                inserted.*,
                p.id as id,
                p.name as name,
                p.price_cents as price_cents,
                p.price_coffee_stamps as price_coffee_stamps,
                p.price_bottle_stamps as price_bottle_stamps,
                p.bonus_cents as bonus_cents,
                p.bonus_coffee_stamps as bonus_coffee_stamps,
                p.bonus_bottle_stamps as bonus_bottle_stamps,
                p.nickname as nickname,
                p.purchase_tax as purchase_tax,
                NULL as image,
                NULL as image_mimetype,
                p.barcode as barcode,
                p.category as category,
                p.print_lists as print_lists,
                p.tags as tags,
                coalesce(array_agg(account_status.id) FILTER (where account_status.id IS NOT NULL), '{}') as status_id,
                coalesce(array_agg(account_status.name) FILTER (where account_status.name IS NOT NULL), '{}') as status_name,
                coalesce(array_agg(account_status.color) FILTER (where account_status.color IS NOT NULL), '{}') as status_color,
                coalesce(array_agg(account_status.priority) FILTER (where account_status.priority IS NOT NULL), '{}') as status_priority,
                coalesce(array_agg(product_status_price.price_cents) FILTER (where product_status_price.price_cents IS NOT NULL), '{}') as status_price_cents,
                coalesce(array_agg(product_status_price.price_bottle_stamps) FILTER (where product_status_price.price_bottle_stamps IS NOT NULL), '{}') as status_price_bottle_stamps,
                coalesce(array_agg(product_status_price.price_coffee_stamps) FILTER (where product_status_price.price_coffee_stamps IS NOT NULL), '{}') as status_price_coffee_stamps,
                coalesce(array_agg(product_status_price.bonus_cents) FILTER (where product_status_price.bonus_cents IS NOT NULL), '{}') as status_bonus_cents,
                coalesce(array_agg(product_status_price.bonus_bottle_stamps) FILTER (where product_status_price.bonus_bottle_stamps IS NOT NULL), '{}') as status_bonus_bottle_stamps,
                coalesce(array_agg(product_status_price.bonus_coffee_stamps) FILTER (where product_status_price.bonus_coffee_stamps IS NOT NULL), '{}') as status_bonus_coffee_stamps
            FROM
                inserted
                    LEFT OUTER JOIN product p ON inserted.product_id = p.id
                    LEFT OUTER JOIN product_status_price ON p.id = product_status_price.product_id
                    LEFT OUTER JOIN account_status on product_status_price.status_id = account_status.id
            GROUP BY inserted.transaction_item_id, inserted.product_id, inserted.transaction_id, inserted.timestamp, inserted.account_id,
                    inserted.effective_price_cents, inserted.effective_price_coffee_stamps, inserted.effective_price_bottle_stamps,
                    inserted.authorized_by_account_id, inserted.authorized_with_method,
                    p.id, p.name,
                    p.price_cents, p.price_coffee_stamps, p.price_bottle_stamps,
                    p.bonus_cents, p.bonus_coffee_stamps, p.bonus_bottle_stamps,
                    p.nickname, p.purchase_tax, p.barcode, p.category, p.print_lists, p.tags
            "#,
        )
        .bind(timestamp)
        .bind(i64::try_from(payment.account).expect("id less than 2**63"))
        .bind(get_type_amounts(CoinType::Cent, &payment.items))
        .bind(get_type_amounts(CoinType::BottleStamp, &payment.items))
        .bind(get_type_amounts(CoinType::CoffeeStamp, &payment.items))
        .bind(
            payment
                .items
                .iter()
                .map(|i| {
                    i.product_id
                        .map(|v| i64::try_from(v).expect("ids are less than 2**63"))
                })
                .collect::<Vec<_>>(),
        )
        .bind(total_price_cents)
        .bind(total_price_bottle_stamps)
        .bind(total_price_coffee_stamps)
        .bind(
            payment
                .authorization
                .as_ref()
                .map(|session| i64::try_from(session.account.id).expect("id less than 2**63")),
        )
        .bind(
            payment
                .authorization
                .map(|ref session| AuthMethodTypeDto::from(session.auth_method)),
        )
        .fetch_all(transaction.as_mut())
        .await;

        to_service_result(transaction.commit().await)?;

        let mut tx = None;
        for row in to_service_result(r)? {
            let new_tx = extend_transaction_with_row(tx.as_mut(), row)?;
            if new_tx.is_some() {
                assert!(tx.is_none(), "inserted only one tx");
                tx = new_tx;
            };
        }

        ServiceResult::Ok(tx.expect("inserted one TX"))
    }

    pub async fn get_all_register_histories(
        &mut self,
    ) -> ServiceResult<Vec<models::RegisterHistory>> {
        let mut r = sqlx::query_as::<_, RegisterHistoryRow>(
            r#"
            SELECT
                id,
                timestamp,
                data
            FROM register_history
            "#,
        )
        .fetch(self.connection.as_mut());

        let mut out = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;
            out.push(row.into());
        }

        Ok(out)
    }

    pub async fn get_register_history_by_id(
        &mut self,
        id: u64,
    ) -> ServiceResult<Option<models::RegisterHistory>> {
        let r = sqlx::query_as::<_, RegisterHistoryRow>(
            r#"
            SELECT
                id,
                timestamp,
                data
            FROM register_history
            WHERE
                register_history.id = $1
            "#,
        )
        .bind(i64::try_from(id).expect("ids are less than 2**63"))
        .fetch_optional(self.connection.as_mut())
        .await;

        Ok(to_service_result(r)?.map(models::RegisterHistory::from))
    }

    pub async fn store_register_history(
        &mut self,
        mut register_history: models::RegisterHistory,
    ) -> ServiceResult<models::RegisterHistory> {
        let q = if register_history.id == 0 {
            sqlx::query(
                r#"
            INSERT INTO register_history (
                timestamp,
                data
            ) VALUES (
                $1,
                $2
            ) RETURNING id
            "#,
            )
        } else {
            sqlx::query(
                r#"
                UPDATE register_history
                SET
                    timestamp = $2,
                    data = $3
                WHERE id = $1
                RETURNING id
            "#,
            )
            .bind(i64::try_from(register_history.id).expect("product id is less than 2**63"))
        };
        let r = q
            .bind(register_history.timestamp)
            .bind(
                serde_json::to_value(RegisterHistoryRowData {
                    source_register: register_history.source_register.into(),
                    target_register: register_history.target_register.into(),
                    envelope_register: register_history.envelope_register.into(),
                })
                .expect("to json cannot fail"),
            )
            .fetch_one(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;

        register_history.id = r
            .get::<i64, _>("id")
            .try_into()
            .expect("id is always positive");
        Ok(register_history)
    }

    pub async fn delete_register_history(&mut self, id: u64) -> ServiceResult<()> {
        let id = i64::try_from(id).expect("id is always less than 2**63");
        let r = sqlx::query(r#"DELETE FROM register_history WHERE id = $1"#)
            .bind(id)
            .execute(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;
        if r.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    pub async fn get_apple_wallet_pass(
        &mut self,
        account_id: u64,
        pass_type_id: &str,
    ) -> ServiceResult<Option<AppleWalletPass>> {
        let r = sqlx::query_as::<_, AppleWalletPassRow>(
            r#"
            SELECT
                account_id,
                pass_type_id,
                CAST(authentication_token AS TEXT),
                qr_code,
                updated_at
            FROM apple_wallet_pass
            WHERE
                apple_wallet_pass.account_id = $1 AND apple_wallet_pass.pass_type_id = $2
            "#,
        )
        .bind(i64::try_from(account_id).expect("ids are less than 2**63"))
        .bind(pass_type_id)
        .fetch_optional(self.connection.as_mut())
        .await;

        Ok(to_service_result(r)?.map(models::AppleWalletPass::from))
    }

    pub async fn list_passes_for_device(
        &mut self,
        pass_type_id: &str,
        device_id: &str,
    ) -> ServiceResult<Vec<AppleWalletPass>> {
        let mut r = sqlx::query_as::<_, AppleWalletPassRow>(
            r#"
            SELECT
                account_id,
                pass_type_id,
                CAST(authentication_token AS TEXT),
                qr_code,
                updated_at
            FROM apple_wallet_pass
            WHERE
                apple_wallet_pass.pass_type_id = $1 AND apple_wallet_pass.device_id = $2
            "#,
        )
        .bind(pass_type_id)
        .bind(device_id)
        .fetch(self.connection.as_mut());

        let mut out = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;
            out.push(row.into());
        }

        Ok(out)
    }

    pub async fn store_apple_wallet_pass(
        &mut self,
        mut pass: AppleWalletPass,
    ) -> ServiceResult<AppleWalletPass> {
        let r = sqlx::query(
            r#"
            INSERT INTO apple_wallet_pass (
                account_id,
                pass_type_id,
                qr_code,
                updated_at
            ) VALUES (
                $1,
                $2,
                $3,
                $4
            ) ON CONFLICT (
                account_id,
                pass_type_id
            ) DO UPDATE SET
                qr_code = $3,
                updated_at = $4
            RETURNING CAST(authentication_token AS TEXT)
            "#,
        )
        .bind(i64::try_from(pass.account_id).expect("ids are less than 2**63"))
        .bind(&pass.pass_type_id)
        .bind(&pass.qr_code)
        .bind(i64::try_from(pass.updated_at).expect("ids are less than 2**63"))
        .fetch_one(self.connection.as_mut())
        .await;
        let r = to_service_result(r)?;

        pass.authentication_token = r.get::<String, _>("authentication_token");
        Ok(pass)
    }

    #[allow(dead_code)]
    pub async fn delete_apple_wallet_pass(
        &mut self,
        account_id: u64,
        pass_type_id: &str,
    ) -> ServiceResult<()> {
        let account_id = i64::try_from(account_id).expect("id is always less than 2**63");
        let r = sqlx::query(
            r#"DELETE FROM apple_wallet_pass WHERE account_id = $1 AND pass_type_id = $2"#,
        )
        .bind(account_id)
        .bind(pass_type_id)
        .execute(self.connection.as_mut())
        .await;
        let r = to_service_result(r)?;
        if r.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    pub async fn get_apple_wallet_registration(
        &mut self,
        account_id: u64,
        pass_type_id: &str,
        device_id: &str,
    ) -> ServiceResult<Option<AppleWalletRegistration>> {
        let r = sqlx::query_as::<_, AppleWalletRegistrationRow>(
            r#"
            SELECT
                account_id,
                pass_type_id,
                device_id,
                push_token
            FROM apple_wallet_registration
            WHERE
                apple_wallet_registration.account_id = $1 AND apple_wallet_registration.pass_type_id = $2 AND apple_wallet_registration.device_id = $3
            "#,
        )
        .bind(i64::try_from(account_id).expect("ids are less than 2**63"))
        .bind(pass_type_id)
        .bind(device_id)
        .fetch_optional(self.connection.as_mut())
        .await;

        Ok(to_service_result(r)?.map(models::AppleWalletRegistration::from))
    }
    pub async fn list_apple_wallet_registration(
        &mut self,
        account_id: u64,
        pass_type_id: &str,
    ) -> ServiceResult<Vec<AppleWalletRegistration>> {
        let mut r = sqlx::query_as::<_, AppleWalletRegistrationRow>(
            r#"
            SELECT
                account_id,
                pass_type_id,
                device_id,
                push_token
            FROM apple_wallet_registration
            WHERE
                apple_wallet_registration.account_id = $1 AND apple_wallet_registration.pass_type_id = $2
            "#,
        )
        .bind(i64::try_from(account_id).expect("ids are less than 2**63"))
        .bind(pass_type_id)
        .fetch(self.connection.as_mut());

        let mut out = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;
            out.push(row.into());
        }

        Ok(out)
    }

    pub async fn store_apple_wallet_registration(
        &mut self,
        registration: AppleWalletRegistration,
    ) -> ServiceResult<AppleWalletRegistration> {
        let r = sqlx::query(
            r#"
            INSERT INTO apple_wallet_registration (
                account_id,
                pass_type_id,
                device_id,
                push_token
            ) VALUES (
                $1,
                $2,
                $3,
                $4
            ) ON CONFLICT (
                account_id,
                pass_type_id,
                device_id
            ) DO UPDATE SET
                push_token = $4
            "#,
        )
        .bind(i64::try_from(registration.account_id).expect("ids are less than 2**63"))
        .bind(&registration.pass_type_id)
        .bind(&registration.device_id)
        .bind(&registration.push_token)
        .fetch_one(self.connection.as_mut())
        .await;
        let r = to_service_result(r)?;

        Ok(registration)
    }

    pub async fn delete_apple_wallet_registration(
        &mut self,
        account_id: u64,
        pass_type_id: &str,
        device_id: &str,
    ) -> ServiceResult<()> {
        let account_id = i64::try_from(account_id).expect("id is always less than 2**63");
        let r = sqlx::query(r#"DELETE FROM apple_wallet_registration WHERE account_id = $1 AND pass_type_id = $2 AND device_id = $3"#)
            .bind(account_id)
            .bind(pass_type_id)
            .bind(device_id)
            .execute(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;
        if r.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    async fn load_purchase_items(
        &mut self,
        purchases: &mut [models::Purchase],
    ) -> ServiceResult<()> {
        let purchase_ids: Vec<i64> = purchases
            .iter()
            .map(|p| i64::try_from(p.id).expect("id is always less than 2**63"))
            .collect();

        let mut r = sqlx::query(
            r#"
                SELECT
                    item.id as purchase_item_id,
                    item.purchase_id as purchase_item_purchase_id,
                    item.name as purchase_item_name,
                    item.container_size as purchase_item_container_size,
                    item.container_count as purchase_item_container_count,
                    item.container_cents as purchase_item_container_cents,
                    p.id as id,
                    p.name as name,
                    p.price_cents as price_cents,
                    p.price_coffee_stamps as price_coffee_stamps,
                    p.price_bottle_stamps as price_bottle_stamps,
                    p.bonus_cents as bonus_cents,
                    p.bonus_coffee_stamps as bonus_coffee_stamps,
                    p.bonus_bottle_stamps as bonus_bottle_stamps,
                    p.nickname as nickname,
                    p.purchase_tax as purchase_tax,
                    NULL as image,
                    NULL as image_mimetype,
                    p.barcode as barcode,
                    p.category as category,
                    p.print_lists as print_lists,
                    p.tags as tags,
                    coalesce(array_agg(account_status.id) FILTER (where account_status.id IS NOT NULL), '{}') as status_id,
                    coalesce(array_agg(account_status.name) FILTER (where account_status.name IS NOT NULL), '{}') as status_name,
                    coalesce(array_agg(account_status.color) FILTER (where account_status.color IS NOT NULL), '{}') as status_color,
                    coalesce(array_agg(account_status.priority) FILTER (where account_status.priority IS NOT NULL), '{}') as status_priority,
                    coalesce(array_agg(product_status_price.price_cents) FILTER (where product_status_price.price_cents IS NOT NULL), '{}') as status_price_cents,
                    coalesce(array_agg(product_status_price.price_bottle_stamps) FILTER (where product_status_price.price_bottle_stamps IS NOT NULL), '{}') as status_price_bottle_stamps,
                    coalesce(array_agg(product_status_price.price_coffee_stamps) FILTER (where product_status_price.price_coffee_stamps IS NOT NULL), '{}') as status_price_coffee_stamps,
                    coalesce(array_agg(product_status_price.bonus_cents) FILTER (where product_status_price.bonus_cents IS NOT NULL), '{}') as status_bonus_cents,
                    coalesce(array_agg(product_status_price.bonus_bottle_stamps) FILTER (where product_status_price.bonus_bottle_stamps IS NOT NULL), '{}') as status_bonus_bottle_stamps,
                    coalesce(array_agg(product_status_price.bonus_coffee_stamps) FILTER (where product_status_price.bonus_coffee_stamps IS NOT NULL), '{}') as status_bonus_coffee_stamps
                FROM
                    purchase_item item
                        LEFT OUTER JOIN product p ON item.product_id = p.id
                        LEFT OUTER JOIN product_status_price ON p.id = product_status_price.product_id
                        LEFT OUTER JOIN account_status on product_status_price.status_id = account_status.id
                WHERE
                    item.purchase_id = ANY($1::BIGINT[])
                GROUP BY item.id, p.id
            "#,
        )
        .bind(purchase_ids)
        .fetch(self.connection.as_mut());

        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;
            let purchase_item_row = to_service_result(PurchaseItemRow::from_row(&row))?;
            let mut purchase_item = models::PurchaseItem {
                id: purchase_item_row.purchase_item_id,
                name: purchase_item_row.purchase_item_name,
                container_cents: purchase_item_row.purchase_item_container_cents,
                container_count: purchase_item_row.purchase_item_container_count,
                container_size: purchase_item_row.purchase_item_container_size,
                product: None,
            };

            let product_id: Option<i64> = row.get("id");
            if product_id.is_some() {
                purchase_item.product = Some(to_service_result(ProductRow::from_row(&row))?.into());
            }

            let purchase = purchases
                .iter_mut()
                .find(|p| p.id == purchase_item_row.purchase_item_purchase_id);

            if let Some(purchase) = purchase {
                purchase.items.push(purchase_item);
            }
        }

        Ok(())
    }

    pub async fn get_purchases(&mut self) -> ServiceResult<Vec<models::Purchase>> {
        let accounts = self.get_all_accounts().await?;

        let mut r = sqlx::query_as::<_, PurchaseRow>(
            r#"
                SELECT
                    id, purchased_by_account_id, store, timestamp
                FROM purchase
            "#,
        )
        .fetch(self.connection.as_mut());

        let mut out = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;
            out.push(row.into());
        }

        drop(r);

        self.load_purchase_items(&mut out).await?;

        Ok(out)
    }

    pub async fn get_purchase_by_id(&mut self, id: u64) -> ServiceResult<Option<models::Purchase>> {
        let accounts = self.get_all_accounts().await?;

        let r = sqlx::query_as::<_, PurchaseRow>(
            r#"
                SELECT id, purchased_by_account_id, store, timestamp
                FROM purchase
                WHERE id = $1
            "#,
        )
        .bind(i64::try_from(id).expect("purchase id is less than 2**63"))
        .fetch_optional(self.connection.as_mut())
        .await;

        let r = to_service_result(r)?;

        let out = if let Some(row) = r {
            let purchase_row: models::Purchase = row.into();
            let mut purchases = vec![purchase_row];
            self.load_purchase_items(&mut purchases).await?;
            purchases.pop()
        } else {
            None
        };

        Ok(out)
    }

    pub async fn get_purchases_by_product_id(
        &mut self,
        product_id: u64,
    ) -> ServiceResult<Option<Vec<models::Purchase>>> {
        if self.get_product_by_id(product_id).await?.is_none() {
            return Ok(None);
        }

        let accounts = self.get_all_accounts().await?;

        let mut r = sqlx::query_as::<_, PurchaseRow>(
            r#"
                SELECT purchase.id, purchased_by_account_id, store, timestamp
                FROM purchase LEFT JOIN purchase_item
                ON purchase.id = purchase_item.purchase_id
                WHERE purchase_item.product_id = $1
                GROUP BY purchase.id, purchased_by_account_id, store, timestamp
            "#,
        )
        .bind(i64::try_from(product_id).expect("purchase id is less than 2**63"))
        .fetch(self.connection.as_mut());

        let mut out = Vec::new();
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;
            out.push(row.into());
        }

        drop(r);

        self.load_purchase_items(&mut out).await?;

        Ok(Some(out))
    }

    pub async fn store_purchase(
        &mut self,
        mut purchase: models::Purchase,
    ) -> ServiceResult<models::Purchase> {
        let q = if purchase.id == 0 {
            sqlx::query(
                r#"
            INSERT INTO purchase (
                purchased_by_account_id,
                store,
                timestamp
            ) VALUES (
                $1,
                $2,
                $3
            ) RETURNING id
            "#,
            )
        } else {
            sqlx::query(
                r#"
                WITH
                    delete AS (DELETE FROM purchase_item WHERE purchase_id = $1)
                UPDATE purchase
                SET
                    purchased_by_account_id = $2,
                    store = $3,
                    timestamp = $4
                WHERE id = $1
                RETURNING id
            "#,
            )
            .bind(i64::try_from(purchase.id).expect("product id is less than 2**63"))
        };
        let r = q
            .bind(
                purchase
                    .purchased_by_account_id
                    .map(|id| i64::try_from(id).expect("id less than 2**63")),
            )
            .bind(purchase.store.as_str())
            .bind(purchase.timestamp)
            .fetch_one(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;

        let purchase_id = r.get::<i64, _>(0);
        purchase.id = purchase_id.try_into().expect("id is always positive");

        let r = sqlx::query(
            r#"
            INSERT INTO purchase_item (purchase_id, name, container_size, container_count, container_cents, product_id)
            SELECT $1, name, container_size, container_count, container_cents, product_id
            FROM UNNEST($2, $3, $4, $5, $6) AS input (name, container_size, container_count, container_cents, product_id)
        "#,
        )
        .bind(purchase_id)
        .bind(
            purchase
                .items
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
        )
        .bind(
            purchase
                .items
                .iter()
                .map(|p| p.container_size)
                .collect::<Vec<_>>(),
        )
        .bind(
            purchase
                .items
                .iter()
                .map(|p| p.container_count)
                .collect::<Vec<_>>(),
        )
        .bind(
            purchase
                .items
                .iter()
                .map(|p| p.container_cents)
                .collect::<Vec<_>>(),
        )
        .bind(
            purchase
                .items
                .iter()
                .map(|p| p.product.as_ref().map(|p| i64::try_from(p.id).expect("id less than 2**63")))
                .collect::<Vec<_>>(),
        )
        .execute(self.connection.as_mut())
        .await;
        to_service_result(r)?;

        Ok(purchase)
    }

    pub async fn delete_purchase(&mut self, id: u64) -> ServiceResult<()> {
        let id = i64::try_from(id).expect("id is always less than 2**63");
        let r = sqlx::query(r#"DELETE FROM purchase WHERE id = $1"#)
            .bind(id)
            .execute(self.connection.as_mut())
            .await;
        let r = to_service_result(r)?;
        if r.rows_affected() != 1 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }
}

fn extend_transaction_with_row(
    prev: Option<&mut Transaction>,
    row: PgRow,
) -> ServiceResult<Option<Transaction>> {
    let mut new_tx = None;
    let txrow = to_service_result(TransactionRow::from_row(&row))?;
    let tx = match prev {
        Some(tx) if tx.id == txrow.transaction_id => tx,
        _ => {
            new_tx = Some(Transaction {
                id: txrow.transaction_id,
                account: u64::try_from(txrow.account_id.unwrap_or(0))
                    .expect("id is always non-negative"),
                timestamp: txrow.timestamp,
                authorized_by_account_id: txrow
                    .authorized_by_account_id
                    .map(|id| u64::try_from(id).expect("id is always non-negative")),
                authorized_with_method: txrow.authorized_with_method.map(|m| m.into()),
                items: Vec::new(),
            });
            new_tx.as_mut().expect("just constructed as Some")
        }
    };

    let mut item = TransactionItem {
        effective_price: to_coin_amount(&[
            (CoinType::Cent, Some(txrow.item.effective_price_cents)),
            (
                CoinType::BottleStamp,
                Some(txrow.item.effective_price_bottle_stamps),
            ),
            (
                CoinType::CoffeeStamp,
                Some(txrow.item.effective_price_coffee_stamps),
            ),
        ]),
        product: None,
    };

    let product_id: Option<i64> = row.get("id");
    if product_id.is_some() {
        item.product = Some(to_service_result(ProductRow::from_row(&row))?.into());
    }

    tx.items.push(item);

    Ok(new_tx)
}
