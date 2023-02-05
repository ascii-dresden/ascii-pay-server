use std::collections::HashMap;

use aide::axum::routing::{get_with, post_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::models;
use crate::request_state::RequestState;

use super::password_hash_create;

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
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
        .api_route(
            "/public-tab-board",
            get_with(
                list_accounts_for_public_tab_board,
                list_accounts_for_public_tab_board_docs,
            ),
        )
        .api_route(
            "/create-admin-account",
            post_with(create_admin_account, create_admin_account_docs),
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
    state.session_require_admin()?;

    let accounts = state.db.get_all_accounts().await?;
    Ok(Json(accounts.iter().map(|a| a.into()).collect()))
}

fn list_accounts_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all accounts.")
        .tag("accounts")
        .response::<200, Json<Vec<AccountDto>>>()
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}

async fn list_accounts_for_public_tab_board(
    mut state: RequestState,
) -> ServiceResult<Json<Vec<AccountDto>>> {
    let accounts = state.db.get_all_accounts().await?;
    let accounts = accounts
        .into_iter()
        .filter(|a| {
            a.auth_methods
                .iter()
                .any(|m| matches!(m, models::AuthMethod::PublicTab))
        })
        .map(|ref a| a.into())
        .collect();
    Ok(Json(accounts))
}

fn list_accounts_for_public_tab_board_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all accounts that participate at the public tab board.")
        .tag("accounts")
        .response::<200, Json<Vec<AccountDto>>>()
}

pub async fn get_account(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<AccountDto>> {
    state.session_require_admin_or_self(id)?;

    let account = state.db.get_account_by_id(id).await?;

    if let Some(account) = account {
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

fn get_account_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get an account by id.")
        .tag("accounts")
        .response::<200, Json<AccountDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
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
    state.session_require_admin()?;

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
        .tag("accounts")
        .response::<200, Json<AccountDto>>()
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}

async fn update_account(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SaveAccountDto>,
) -> ServiceResult<Json<AccountDto>> {
    state.session_require_admin_or_self(id)?;

    let form = form.0;
    let account = state.db.get_account_by_id(id).await?;

    if let Some(mut account) = account {
        account.name = form.name;
        account.email = form.email;

        let new_role = form.role.into();
        if account.role != new_role {
            // Only admins are allowed to change account roles
            state.session_require_admin()?;
            account.role = new_role;
        }

        let account = state.db.store_account(account).await?;
        return Ok(Json(AccountDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

fn update_account_docs(op: TransformOperation) -> TransformOperation {
    op.description("Update an existing account.")
        .tag("accounts")
        .response::<200, Json<AccountDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

async fn delete_account(mut state: RequestState, Path(id): Path<u64>) -> ServiceResult<StatusCode> {
    state.session_require_admin_or_self(id)?;

    state.db.delete_account(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_account_docs(op: TransformOperation) -> TransformOperation {
    op.description("Delete an existing account.")
        .tag("accounts")
        .response_with::<204, (), _>(|res| res.description("The account was successfully deleted!"))
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct CreateAdminAccountDto {
    pub name: String,
    pub email: String,
    pub username: String,
    pub password: String,
}

async fn create_admin_account(
    mut state: RequestState,
    form: Json<CreateAdminAccountDto>,
) -> ServiceResult<Json<AccountDto>> {
    let accounts = state.db.get_all_accounts().await?;
    let admin_account_found = accounts
        .iter()
        .any(|a| matches!(a.role, models::Role::Admin));
    if admin_account_found {
        return Err(ServiceError::NotFound);
    }

    let form = form.0;

    let mut account = models::Account {
        id: 0,
        balance: models::CoinAmount(HashMap::new()),
        name: form.name,
        email: form.email,
        role: models::Role::Admin,
        auth_methods: Vec::new(),
    };

    account
        .auth_methods
        .push(models::AuthMethod::PasswordBased(models::AuthPassword {
            username: form.username,
            password_hash: password_hash_create(&form.password)?,
        }));

    let account = state.db.store_account(account).await?;
    Ok(Json(AccountDto::from(&account)))
}

fn create_admin_account_docs(op: TransformOperation) -> TransformOperation {
    op.description("Create an initial admin account.")
        .tag("accounts")
        .response::<200, Json<AccountDto>>()
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}
