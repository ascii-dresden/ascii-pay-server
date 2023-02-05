use aide::axum::routing::{post_with, put_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use argon2rs::verifier::Encoded;
use axum::extract::Path;
use axum::Json;
use base64::engine::general_purpose;
use base64::Engine;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::models;
use crate::request_state::RequestState;

use super::accounts::{AccountDto, CardTypeDto};

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
        .with_state(app_state)
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
    state.session_require_admin_or_self(id)?;

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
        .tag("account_authentication")
        .response::<200, Json<AccountDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

async fn delete_password_authentication(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<AccountDto>> {
    state.session_require_admin_or_self(id)?;

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
        .tag("account_authentication")
        .response::<200, Json<AccountDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct CreateAuthNfcDto {
    pub name: String,
    pub card_id: String,
    pub card_type: CardTypeDto,
    pub data: Option<String>,
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
    state.session_require_admin_or_self(id)?;

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
        let data = general_purpose::STANDARD
            .decode(form.data.unwrap_or_default())
            .map_err(|_| {
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
        .tag("account_authentication")
        .response::<200, Json<AccountDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

async fn update_nfc_authentication(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<UpdateAuthNfcDto>,
) -> ServiceResult<Json<AccountDto>> {
    state.session_require_admin_or_self(id)?;

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
        .tag("account_authentication")
        .response::<200, Json<AccountDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

async fn delete_nfc_authentication(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<DeleteAuthNfcDto>,
) -> ServiceResult<Json<AccountDto>> {
    state.session_require_admin_or_self(id)?;

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
        .tag("account_authentication")
        .response::<200, Json<AccountDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

fn password_hash_create(password: &str) -> ServiceResult<Vec<u8>> {
    let bytes =
        Encoded::default2i(password.as_bytes(), "SALTSALTSALT".as_bytes(), b"", b"").to_u8();
    Ok(bytes)
}