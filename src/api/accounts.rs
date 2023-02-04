use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::error::{ServiceError, ServiceResult};
use crate::models;

pub fn router() -> Router<Database> {
    // Router::new()
    //     .route("/account/:id", get(get_account).put(update_account).delete(delete_account))
    //     .route("/accounts", get(list_accounts).post(create_account))

    Router::new()
        .route("/account/:id", get(get_account).put(update_account).delete(delete_account))
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

#[derive(Debug, PartialEq, Serialize)]
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
