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
use sqlx::{FromRow, PgPool, Row};
use sqlx::{Pool, Postgres};
use tokio::sync::Mutex;

use crate::error::{ServiceError, ServiceResult};
use crate::models::{
    self, Account, AuthMethod, AuthMethodType, AuthNfc, AuthPassword, AuthRequest, CardType,
    CoinAmount, CoinType, Image, PaymentItem, Product, Role, Session, Transaction, TransactionItem,
};

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
    Admin,
}

impl From<AccountRoleDto> for Role {
    fn from(value: AccountRoleDto) -> Self {
        match value {
            AccountRoleDto::Basic => Role::Basic,
            AccountRoleDto::Member => Role::Member,
            AccountRoleDto::Admin => Role::Admin,
        }
    }
}

impl From<Role> for AccountRoleDto {
    fn from(value: Role) -> Self {
        match value {
            Role::Basic => AccountRoleDto::Basic,
            Role::Member => AccountRoleDto::Member,
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
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
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
        }
    }
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
            } => AuthMethod::NfcBased(AuthNfc {
                name,
                card_id,
                card_type: card_type.into(),
                data,
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
    nickname: Option<String>,
    #[sqlx(flatten)]
    image: ProductImageRow,
    barcode: Option<String>,
    category: String,
    tags: Vec<String>,
}

impl From<ProductRow> for Product {
    fn from(value: ProductRow) -> Self {
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
            image: value.image.into(),
            barcode: value.barcode,
            category: value.category,
            tags: value.tags,
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
    #[sqlx(flatten)]
    item: TransactionItemRow,
}

impl DatabaseConnection {
    pub async fn get_all_accounts(&mut self) -> ServiceResult<Vec<models::Account>> {
        let mut r = sqlx::query_as::<_, AccountRow>(
            r#"
            SELECT
                a.id, a.balance_cents, a.balance_coffee_stamps, a.balance_bottle_stamps,
                a.name, a.email, a.role,
                coalesce(array_agg(account_auth_method.data ORDER BY account_auth_method.id ASC) FILTER (where account_auth_method.id IS NOT NULL), '{}') AS auth_methods
            FROM account AS a
            LEFT OUTER JOIN account_auth_method ON a.id = account_auth_method.account_id
            GROUP BY a.id
        "#,
        )
        .fetch(&mut self.connection);

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
                coalesce(array_agg(account_auth_method.data ORDER BY account_auth_method.id ASC) FILTER (where account_auth_method.id IS NOT NULL), '{}') AS auth_methods
            FROM account AS a
            LEFT OUTER JOIN account_auth_method ON a.id = account_auth_method.account_id
            WHERE a.id = $1
            GROUP BY a.id
        "#)
        .bind(i64::try_from(id).expect("account id is less than 2**63"))
        .fetch_optional(&mut self.connection)
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
                coalesce(array_agg(account_auth_method.data ORDER BY account_auth_method.id ASC) FILTER (where account_auth_method.id IS NOT NULL), '{}') AS auth_methods
            FROM account AS a INNER JOIN matching ON matching.account_id = a.id
            LEFT OUTER JOIN account_auth_method ON a.id = account_auth_method.account_id
            GROUP BY a.id
        "#)
        .bind(auth_method.login_key())
        .fetch_optional(&mut self.connection)
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
        .fetch_one(&mut self.connection)
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
        .execute(&mut self.connection)
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
        .execute(&mut self.connection)
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
                    coalesce(array_agg(account_auth_method.data ORDER BY account_auth_method.id ASC) FILTER (where account_auth_method.id IS NOT NULL), '{}') AS auth_methods
                    FROM account AS a
                    LEFT OUTER JOIN account_auth_method ON a.id = account_auth_method.account_id
                    GROUP BY a.id
                )
            SELECT CAST(session.uuid as TEXT) as uuid, session.auth_method, session.valid_until, session.is_single_use, full_account.*
            FROM full_account INNER JOIN session on full_account.id = session.account_id
            WHERE session.valid_until > now() AND session.uuid = CAST($1 as UUID)
        "#)
        .bind(session_token)
        .fetch_optional(&mut self.connection)
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
                    coalesce(array_agg(account_auth_method.data ORDER BY account_auth_method.id ASC) FILTER (where account_auth_method.id IS NOT NULL), '{}') AS auth_methods
                    FROM account AS a
                    LEFT OUTER JOIN account_auth_method ON a.id = account_auth_method.account_id
                    GROUP BY a.id
                )
            SELECT CAST(session.uuid as TEXT) as uuid, session.auth_method, session.valid_until, session.is_single_use, full_account.*
            FROM full_account INNER JOIN session on full_account.id = session.account_id
            WHERE session.valid_until > now() AND session.account_id = $1
        "#)
        .bind(account_id)
        .fetch(&mut self.connection);

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
                INSERT INTO account (balance_cents, balance_coffee_stamps, balance_bottle_stamps, name, email, role)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id
            "#,
            )
        } else {
            sqlx::query(r#"
                WITH
                    delete AS (DELETE FROM account_auth_method WHERE account_id = $1)
                UPDATE account
                SET balance_cents = $2, balance_coffee_stamps = $3, balance_bottle_stamps = $4, name = $5, email = $6, role = $7
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
            .fetch_one(&mut self.connection)
            .await;
        let r = to_service_result(r)?;
        let account_id = r.get::<i64, _>(0);
        account.id = account_id.try_into().expect("id is always positive");

        let r = sqlx::query(
            r#"
            INSERT INTO account_auth_method (account_id, login_key, data)
            SELECT $1, login_key, data
            FROM UNNEST($2, $3) AS input (login_key, data)
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
        .execute(&mut self.connection)
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
        .execute(&mut self.connection)
        .await;
        to_service_result(r)?;
        Ok(())
    }

    pub async fn get_all_products(&mut self) -> ServiceResult<Vec<models::Product>> {
        let mut r = sqlx::query_as::<_, ProductRow>(
            r#"
            SELECT
                id,
                name,
                price_cents,
                price_coffee_stamps,
                price_bottle_stamps,
                bonus_cents,
                bonus_coffee_stamps,
                bonus_bottle_stamps,
                nickname,
                NULL AS image,
                NULL AS image_mimetype,
                barcode,
                category,
                tags
            FROM product
            "#,
        )
        .fetch(&mut self.connection);

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
                id,
                name,
                price_cents,
                price_coffee_stamps,
                price_bottle_stamps,
                bonus_cents,
                bonus_coffee_stamps,
                bonus_bottle_stamps,
                nickname,
                NULL AS image,
                NULL AS image_mimetype,
                barcode,
                category,
                tags
            FROM product
            WHERE
                product.id = $1
            "#,
        )
        .bind(i64::try_from(id).expect("ids are less than 2**63"))
        .fetch_optional(&mut self.connection)
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
                barcode,
                category,
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
                $11
            ) RETURNING id
            "#,
            )
        } else {
            sqlx::query(
                r#"
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
                    barcode = $10,
                    category = $11,
                    tags = $12
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
            .bind(&product.barcode)
            .bind(&product.category)
            .bind(&product.tags)
            .fetch_one(&mut self.connection)
            .await;
        let r = to_service_result(r)?;

        product.id = r
            .get::<i64, _>("id")
            .try_into()
            .expect("id is always positive");
        Ok(product)
    }

    pub async fn delete_product(&mut self, id: u64) -> ServiceResult<()> {
        let id = i64::try_from(id).expect("id is always less than 2**63");
        let r = sqlx::query(r#"DELETE FROM product WHERE id = $1"#)
            .bind(id)
            .execute(&mut self.connection)
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
        .fetch_optional(&mut self.connection)
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
            .execute(&mut self.connection)
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
                .execute(&mut self.connection)
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
                    p.id as id,
                    p.name as name,
                    p.price_cents as price_cents,
                    p.price_coffee_stamps as price_coffee_stamps,
                    p.price_bottle_stamps as price_bottle_stamps,
                    p.bonus_cents as bonus_cents,
                    p.bonus_coffee_stamps as bonus_coffee_stamps,
                    p.bonus_bottle_stamps as bonus_bottle_stamps,
                    p.nickname as nickname,
                    NULL as image,
                    NULL as image_mimetype,
                    p.barcode as barcode,
                    p.category as category,
                    p.tags as tags
                FROM
                    transaction_item item
                    LEFT OUTER JOIN product p ON item.product_id = p.id
                ORDER BY item.timestamp ASC, item.transaction_id ASC
            "#,
        )
        .fetch(&mut self.connection);

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
                    p.id as id,
                    p.name as name,
                    p.price_cents as price_cents,
                    p.price_coffee_stamps as price_coffee_stamps,
                    p.price_bottle_stamps as price_bottle_stamps,
                    p.bonus_cents as bonus_cents,
                    p.bonus_coffee_stamps as bonus_coffee_stamps,
                    p.bonus_bottle_stamps as bonus_bottle_stamps,
                    p.nickname as nickname,
                    NULL as image,
                    NULL as image_mimetype,
                    p.barcode as barcode,
                    p.category as category,
                    p.tags as tags
                FROM
                    transaction_item item
                    LEFT OUTER JOIN product p ON item.product_id = p.id
                WHERE
                    (item.account_id = $1) OR ($1 = 0 AND item.account_id IS NULL)
                ORDER BY item.timestamp ASC, item.transaction_id ASC
            "#,
        )
        .bind(id)
        .fetch(&mut self.connection);

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
                p.id as id,
                p.name as name,
                p.price_cents as price_cents,
                p.price_coffee_stamps as price_coffee_stamps,
                p.price_bottle_stamps as price_bottle_stamps,
                p.bonus_cents as bonus_cents,
                p.bonus_coffee_stamps as bonus_coffee_stamps,
                p.bonus_bottle_stamps as bonus_bottle_stamps,
                p.nickname as nickname,
                NULL as image,
                NULL as image_mimetype,
                p.barcode as barcode,
                p.category as category,
                p.tags as tags
            FROM
                transaction_item item
                LEFT OUTER JOIN product p ON item.product_id = p.id
            WHERE item.transaction_id = $1
        "#,
        )
        .bind(id)
        .fetch(&mut self.connection);

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

        let mut r = sqlx::query(
            r#"
            WITH
                transaction_args AS (
                    SELECT
                        nextval('transaction_id_seq') AS transaction_id,
                        $1 AS timestamp,
                        $2 AS account_id
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
                        account_id
                    )
                    SELECT
                        transaction_id,
                        effective_price_cents,
                        effective_price_bottle_stamps,
                        effective_price_coffee_stamps,
                        product_id,
                        timestamp,
                        account_id
                    FROM
                        transaction_args,
                        UNNEST($3, $4, $5, $6) AS item_args(
                            effective_price_cents,
                            effective_price_bottle_stamps,
                            effective_price_coffee_stamps,
                            product_id
                        )
                    RETURNING
                        transaction_id,
                        effective_price_cents,
                        effective_price_bottle_stamps,
                        effective_price_coffee_stamps,
                        product_id,
                        timestamp,
                        account_id
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
                NULL as image,
                NULL as image_mimetype,
                p.barcode as barcode,
                p.category as category,
                p.tags as tags
            FROM inserted LEFT OUTER JOIN product p ON p.id = inserted.product_id
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
        .fetch(&mut self.connection);

        let mut tx = None;
        while let Some(row) = r.next().await {
            let row = to_service_result(row)?;
            let new_tx = extend_transaction_with_row(tx.as_mut(), row)?;
            if new_tx.is_some() {
                assert!(tx.is_none(), "inserted only one tx");
                tx = new_tx;
            };
        }

        Ok(tx.expect("inserted one TX"))
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
