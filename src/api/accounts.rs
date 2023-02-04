use std::collections::HashMap;

use argon2rs::verifier::Encoded;
use axum::extract::{Path, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::engine::general_purpose;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::error::{ServiceError, ServiceResult};
use crate::models;

pub fn router() -> Router<Database> {
    Router::new()
        .route(
            "/account/:id/password_authentication",
            put(set_password_authentication).delete(delete_password_authentication),
        )
        .route(
            "/account/:id/nfc_authentication",
            post(create_nfc_authentication)
                .put(update_nfc_authentication)
                .delete(delete_nfc_authentication),
        )
        .route(
            "/account/:id",
            get(get_account).put(update_account).delete(delete_account),
        )
        .route("/accounts", get(list_accounts).post(create_account))
}

#[derive(Debug, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub enum CoinTypeDto {
    Cent,
    CoffeeStamp,
    BottleStamp,
}

impl From<&models::CoinType> for CoinTypeDto {
    fn from(value: &models::CoinType) -> Self {
        match value {
            models::CoinType::Cent => CoinTypeDto::Cent,
            models::CoinType::CoffeeStamp => CoinTypeDto::CoffeeStamp,
            models::CoinType::BottleStamp => CoinTypeDto::BottleStamp,
        }
    }
}

impl From<CoinTypeDto> for models::CoinType {
    fn from(value: CoinTypeDto) -> Self {
        match value {
            CoinTypeDto::Cent => models::CoinType::Cent,
            CoinTypeDto::CoffeeStamp => models::CoinType::CoffeeStamp,
            CoinTypeDto::BottleStamp => models::CoinType::BottleStamp,
        }
    }
}

pub type CoinAmountDto = HashMap<CoinTypeDto, i32>;
impl From<&models::CoinAmount> for CoinAmountDto {
    fn from(value: &models::CoinAmount) -> Self {
        let mut map = HashMap::<CoinTypeDto, i32>::new();

        for (coin_type, amount) in value.0.iter() {
            map.insert(coin_type.into(), *amount);
        }

        map
    }
}
impl From<CoinAmountDto> for models::CoinAmount {
    fn from(value: CoinAmountDto) -> Self {
        let mut map = HashMap::<models::CoinType, i32>::new();

        for (coin_type, amount) in value.into_iter() {
            map.insert(coin_type.into(), amount);
        }

        models::CoinAmount(map)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum RoleDto {
    Basic,
    Member,
    Admin,
}
impl From<&models::Role> for RoleDto {
    fn from(value: &models::Role) -> Self {
        match value {
            models::Role::Basic => RoleDto::Basic,
            models::Role::Member => RoleDto::Member,
            models::Role::Admin => RoleDto::Admin,
        }
    }
}
impl From<RoleDto> for models::Role {
    fn from(value: RoleDto) -> Self {
        match value {
            RoleDto::Basic => models::Role::Basic,
            RoleDto::Member => models::Role::Member,
            RoleDto::Admin => models::Role::Admin,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum CardTypeDto {
    NfcId,
    AsciiMifare,
}
impl From<&models::CardType> for CardTypeDto {
    fn from(value: &models::CardType) -> Self {
        match value {
            models::CardType::NfcId => CardTypeDto::NfcId,
            models::CardType::AsciiMifare => CardTypeDto::AsciiMifare,
        }
    }
}
impl From<CardTypeDto> for models::CardType {
    fn from(value: CardTypeDto) -> Self {
        match value {
            CardTypeDto::NfcId => models::CardType::NfcId,
            CardTypeDto::AsciiMifare => models::CardType::AsciiMifare,
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct AuthPasswordDto {
    username: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct AuthNfcDto {
    name: String,
    card_id: Vec<u8>,
    card_type: CardTypeDto,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum AuthMethodDto {
    PasswordBased(AuthPasswordDto),
    NfcBased(AuthNfcDto),
    PublicTab,
}
impl From<&models::AuthMethod> for AuthMethodDto {
    fn from(value: &models::AuthMethod) -> Self {
        match value {
            models::AuthMethod::PasswordBased(password_based) => {
                AuthMethodDto::PasswordBased(AuthPasswordDto {
                    username: password_based.username.to_owned(),
                })
            }
            models::AuthMethod::NfcBased(nfc_based) => AuthMethodDto::NfcBased(AuthNfcDto {
                name: nfc_based.name.to_owned(),
                card_id: nfc_based.card_id.to_owned(),
                card_type: (&nfc_based.card_type).into(),
            }),
            models::AuthMethod::PublicTab => AuthMethodDto::PublicTab,
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct AccountDto {
    pub id: u64,
    pub balance: CoinAmountDto,
    pub name: String,
    pub email: String,
    pub role: RoleDto,
    pub auth_methods: Vec<AuthMethodDto>,
}

impl From<&models::Account> for AccountDto {
    fn from(value: &models::Account) -> Self {
        let mut auth_methods = Vec::<AuthMethodDto>::new();

        for auth_method in value.auth_methods.iter() {
            auth_methods.push(auth_method.into());
        }

        Self {
            id: value.id.to_owned(),
            balance: (&value.balance).into(),
            name: value.name.to_owned(),
            email: value.email.to_owned(),
            role: (&value.role).into(),
            auth_methods,
        }
    }
}

pub async fn list_accounts(
    State(database): State<Database>,
) -> ServiceResult<Json<Vec<AccountDto>>> {
    let accounts = database.get_all_accounts().await?;
    let mut account_list = Vec::<AccountDto>::new();

    for account in accounts.iter() {
        account_list.push(account.into());
    }

    Ok(Json(account_list))
}

pub async fn get_account(
    State(database): State<Database>,
    Path(id): Path<u64>,
) -> ServiceResult<Json<AccountDto>> {
    let account = database.get_account_by_id(id).await?;

    if let Some(account) = account {
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct SaveAccountDto {
    pub name: String,
    pub email: String,
    pub role: RoleDto,
}

async fn create_account(
    State(database): State<Database>,
    form: Json<SaveAccountDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;

    let account = models::Account {
        id: 0,
        balance: models::CoinAmount(HashMap::new()),
        name: form.name,
        email: form.email,
        role: form.role.into(),
        auth_methods: Vec::new(),
    };

    let account = database.store_account(account).await?;
    Ok(Json(AccountDto::from(&account)))
}

async fn update_account(
    State(database): State<Database>,
    Path(id): Path<u64>,
    form: Json<SaveAccountDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;
    let account = database.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        account.name = form.name;
        account.email = form.email;
        account.role = form.role.into();

        let account = database.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

async fn delete_account(
    State(database): State<Database>,
    Path(id): Path<u64>,
) -> ServiceResult<()> {
    database.delete_account(id).await
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct SaveAuthPasswordDto {
    pub username: String,
    pub password: String,
}

async fn set_password_authentication(
    State(database): State<Database>,
    Path(id): Path<u64>,
    form: Json<SaveAuthPasswordDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;
    let account = database.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        account
            .auth_methods
            .retain_mut(|m| !matches!(m, &mut models::AuthMethod::PasswordBased(_)));
        account
            .auth_methods
            .push(models::AuthMethod::PasswordBased(models::AuthPassword {
                username: form.username,
                password_hash: password_hash_create(&form.password)?,
            }));

        let account = database.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

async fn delete_password_authentication(
    State(database): State<Database>,
    Path(id): Path<u64>,
) -> ServiceResult<Json<AccountDto>> {
    let account = database.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        account
            .auth_methods
            .retain_mut(|m| !matches!(m, &mut models::AuthMethod::PasswordBased(_)));

        let account = database.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct CreateAuthNfcDto {
    pub name: String,
    pub card_id: String,
    pub card_type: CardTypeDto,
    pub data: String,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct UpdateAuthNfcDto {
    pub card_id: String,
    pub name: String,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct DeleteAuthNfcDto {
    pub card_id: String,
}

async fn create_nfc_authentication(
    State(database): State<Database>,
    Path(id): Path<u64>,
    form: Json<CreateAuthNfcDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;
    let account = database.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        let card_id = general_purpose::STANDARD.decode(form.card_id).unwrap();
        let data = general_purpose::STANDARD.decode(form.data).unwrap();

        account
            .auth_methods
            .push(models::AuthMethod::NfcBased(models::AuthNfc {
                name: form.name,
                card_id,
                card_type: form.card_type.into(),
                data,
            }));

        let account = database.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

async fn update_nfc_authentication(
    State(database): State<Database>,
    Path(id): Path<u64>,
    form: Json<UpdateAuthNfcDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;
    let account = database.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        let card_id = general_purpose::STANDARD.decode(form.card_id).unwrap();

        for method in account.auth_methods.iter_mut() {
            if let models::AuthMethod::NfcBased(nfc_based) = method {
                if nfc_based.card_id == card_id {
                    nfc_based.name = form.name.clone();
                }
            }
        }

        let account = database.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

async fn delete_nfc_authentication(
    State(database): State<Database>,
    Path(id): Path<u64>,
    form: Json<DeleteAuthNfcDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;
    let account = database.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        let card_id = general_purpose::STANDARD.decode(form.card_id).unwrap();

        account.auth_methods.retain_mut(|m| {
            if let models::AuthMethod::NfcBased(nfc_based) = m {
                nfc_based.card_id != card_id
            } else {
                true
            }
        });

        let account = database.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

fn password_hash_create(password: &str) -> ServiceResult<Vec<u8>> {
    let bytes =
        Encoded::default2i(password.as_bytes(), "SALTSALTSALT".as_bytes(), b"", b"").to_u8();
    Ok(bytes)
}

fn password_hash_verify(hash: &[u8], password: &str) -> ServiceResult<bool> {
    if let Ok(enc) = Encoded::from_u8(hash) {
        return Ok(enc.verify(password.as_bytes()));
    }

    Ok(false)
}
