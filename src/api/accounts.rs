use std::collections::HashMap;

use aide::axum::routing::{get_with, post_with, put_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use argon2rs::verifier::Encoded;
use axum::extract::Path;
use axum::Json;
use base64::engine::general_purpose;
use base64::Engine;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::models;
use crate::request_state::RequestState;

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/account/:id/password_authentication",
            put_with(
                set_password_authentication,
                set_password_authentication_docs,
            )
            .delete_with(
                delete_password_authentication,
                delete_password_authentication_docs,
            ),
        )
        .api_route(
            "/account/:id/nfc_authentication",
            post_with(create_nfc_authentication, create_nfc_authentication_docs)
                .put_with(update_nfc_authentication, update_nfc_authentication_docs)
                .delete_with(delete_nfc_authentication, delete_nfc_authentication_docs),
        )
        .api_route(
            "/account/:id",
            get_with(get_account, get_account_docs)
                .put_with(update_account, update_account_docs)
                .delete_with(delete_account, delete_account_docs),
        )
        .api_route(
            "/accounts",
            get_with(list_accounts, list_accounts_docs)
                .post_with(create_account, create_account_docs),
        )
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Hash, Eq, Serialize, Deserialize, JsonSchema)]
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
        value
            .0
            .iter()
            .map(|(coin_type, amount)| (coin_type.into(), *amount))
            .collect()
    }
}
impl From<CoinAmountDto> for models::CoinAmount {
    fn from(value: CoinAmountDto) -> Self {
        models::CoinAmount(
            value
                .into_iter()
                .map(|(coin_type, amount)| (coin_type.into(), amount))
                .collect(),
        )
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct AuthPasswordDto {
    username: String,
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct AuthNfcDto {
    name: String,
    card_id: Vec<u8>,
    card_type: CardTypeDto,
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
        Self {
            id: value.id.to_owned(),
            balance: (&value.balance).into(),
            name: value.name.to_owned(),
            email: value.email.to_owned(),
            role: (&value.role).into(),
            auth_methods: value.auth_methods.iter().map(|m| m.into()).collect(),
        }
    }
}

async fn list_accounts(mut state: RequestState) -> ServiceResult<Json<Vec<AccountDto>>> {
    let accounts = state.db.get_all_accounts().await?;
    Ok(Json(accounts.iter().map(|a| a.into()).collect()))
}

fn list_accounts_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all accounts.")
        .response::<200, Json<Vec<AccountDto>>>()
        .response::<500, ()>()
}

pub async fn get_account(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<AccountDto>> {
    let account = state.db.get_account_by_id(id).await?;

    if let Some(account) = account {
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

fn get_account_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get an account by id.")
        .response::<200, Json<AccountDto>>()
        .response::<404, ()>()
        .response::<500, ()>()
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SaveAccountDto {
    pub name: String,
    pub email: String,
    pub role: RoleDto,
}

async fn create_account(
    mut state: RequestState,
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

    let account = state.db.store_account(account).await?;
    Ok(Json(AccountDto::from(&account)))
}

fn create_account_docs(op: TransformOperation) -> TransformOperation {
    op.description("Create a new account.")
        .response::<200, Json<AccountDto>>()
        .response::<500, ()>()
}

async fn update_account(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SaveAccountDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;
    let account = state.db.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        account.name = form.name;
        account.email = form.email;
        account.role = form.role.into();

        let account = state.db.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}
fn update_account_docs(op: TransformOperation) -> TransformOperation {
    op.description("Update an existing account.")
        .response::<200, Json<AccountDto>>()
        .response::<404, ()>()
        .response::<500, ()>()
}

async fn delete_account(mut state: RequestState, Path(id): Path<u64>) -> ServiceResult<()> {
    state.db.delete_account(id).await
}
fn delete_account_docs(op: TransformOperation) -> TransformOperation {
    op.description("Delete an existing account.")
        .response::<200, ()>()
        .response::<404, ()>()
        .response::<500, ()>()
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SaveAuthPasswordDto {
    pub username: String,
    pub password: String,
}

async fn set_password_authentication(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SaveAuthPasswordDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;
    let account = state.db.get_account_by_id(id).await?;

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

        let account = state.db.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}
fn set_password_authentication_docs(op: TransformOperation) -> TransformOperation {
    op.description("Set username and password for the given account.")
        .response::<200, Json<AccountDto>>()
        .response::<404, ()>()
        .response::<500, ()>()
}

async fn delete_password_authentication(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<AccountDto>> {
    let account = state.db.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        account
            .auth_methods
            .retain_mut(|m| !matches!(m, &mut models::AuthMethod::PasswordBased(_)));

        let account = state.db.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}
fn delete_password_authentication_docs(op: TransformOperation) -> TransformOperation {
    op.description("Remove password authentication from the given account.")
        .response::<200, Json<AccountDto>>()
        .response::<404, ()>()
        .response::<500, ()>()
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct CreateAuthNfcDto {
    pub name: String,
    pub card_id: String,
    pub card_type: CardTypeDto,
    pub data: String,
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct UpdateAuthNfcDto {
    pub card_id: String,
    pub name: String,
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct DeleteAuthNfcDto {
    pub card_id: String,
}

async fn create_nfc_authentication(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<CreateAuthNfcDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;
    let account = state.db.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        let card_id = general_purpose::STANDARD
            .decode(form.card_id)
            .map_err(|_| {
                ServiceError::InternalServerError(
                    "Could not decode base64 parameter 'card_id'.".to_string(),
                )
            })?;
        let data = general_purpose::STANDARD.decode(form.data).map_err(|_| {
            ServiceError::InternalServerError(
                "Could not decode base64 parameter 'data'.".to_string(),
            )
        })?;

        account
            .auth_methods
            .push(models::AuthMethod::NfcBased(models::AuthNfc {
                name: form.name,
                card_id,
                card_type: form.card_type.into(),
                data,
            }));

        let account = state.db.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}
fn create_nfc_authentication_docs(op: TransformOperation) -> TransformOperation {
    op.description("Add a new nfc based authentication method to the given account.")
        .response::<200, Json<AccountDto>>()
        .response::<404, ()>()
        .response::<500, ()>()
}

async fn update_nfc_authentication(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<UpdateAuthNfcDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;
    let account = state.db.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        let card_id = general_purpose::STANDARD
            .decode(form.card_id)
            .map_err(|_| {
                ServiceError::InternalServerError(
                    "Could not decode base64 parameter 'card_id'.".to_string(),
                )
            })?;

        for method in account.auth_methods.iter_mut() {
            if let models::AuthMethod::NfcBased(nfc_based) = method {
                if nfc_based.card_id == card_id {
                    nfc_based.name = form.name.clone();
                }
            }
        }

        let account = state.db.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}
fn update_nfc_authentication_docs(op: TransformOperation) -> TransformOperation {
    op.description("Update an existing nfc based authentication method of the given account.")
        .response::<200, Json<AccountDto>>()
        .response::<404, ()>()
        .response::<500, ()>()
}

async fn delete_nfc_authentication(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<DeleteAuthNfcDto>,
) -> ServiceResult<Json<AccountDto>> {
    let form = form.0;
    let account = state.db.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        let card_id = general_purpose::STANDARD
            .decode(form.card_id)
            .map_err(|_| {
                ServiceError::InternalServerError(
                    "Could not decode base64 parameter 'card_id'.".to_string(),
                )
            })?;

        account.auth_methods.retain_mut(|m| {
            if let models::AuthMethod::NfcBased(nfc_based) = m {
                nfc_based.card_id != card_id
            } else {
                true
            }
        });

        let account = state.db.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}
fn delete_nfc_authentication_docs(op: TransformOperation) -> TransformOperation {
    op.description("Remmove an existing nfc based authentication method from the given account.")
        .response::<200, Json<AccountDto>>()
        .response::<404, ()>()
        .response::<500, ()>()
}

fn password_hash_create(password: &str) -> ServiceResult<Vec<u8>> {
    let bytes =
        Encoded::default2i(password.as_bytes(), "SALTSALTSALT".as_bytes(), b"", b"").to_u8();
    Ok(bytes)
}
