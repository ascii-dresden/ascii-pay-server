use std::ops::Add;

use aide::axum::routing::{delete_with, get_with, post_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use aide::OperationOutput;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use base64::engine::general_purpose;
use base64::Engine;
use chrono::{Duration, Utc};
use headers::{HeaderMap, HeaderValue};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::request_state::RequestState;
use crate::{models, SESSION_COOKIE_NAME};

use super::accounts::AccountDto;
use super::{mifare, password_hash_verify};

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/auth/password",
            post_with(auth_password_based, auth_password_based_docs),
        )
        .api_route(
            "/auth/nfc-id",
            post_with(auth_nfc_based_nfc_id, auth_nfc_based_nfc_id_docs),
        )
        .api_route(
            "/auth/ascii-mifare/challenge",
            post_with(
                auth_nfc_based_ascii_mifare_challenge,
                auth_nfc_based_ascii_mifare_challenge_docs,
            ),
        )
        .api_route(
            "/auth/ascii-mifare/response",
            post_with(
                auth_nfc_based_ascii_mifare_response,
                auth_nfc_based_ascii_mifare_response_docs,
            ),
        )
        .api_route(
            "/auth/account",
            get_with(auth_get_account, auth_get_account_docs),
        )
        .api_route("/auth", delete_with(auth_delete, auth_delete_docs))
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct AuthTokenDto {
    pub token: String,
}

impl OperationOutput for AuthTokenDto {
    type Inner = AuthTokenDto;
}
impl IntoResponse for AuthTokenDto {
    fn into_response(self) -> axum::response::Response {
        let cookie = HeaderValue::from_str(
            format!(
                "{}={}; Path=/api/v1; HttpOnly; SameSite=None",
                SESSION_COOKIE_NAME, self.token
            )
            .as_str(),
        )
        .unwrap();

        let mut header = HeaderMap::new();
        header.insert(header::SET_COOKIE, cookie);
        (StatusCode::OK, header, Json(self)).into_response()
    }
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct AuthPasswordBasedDto {
    pub username: String,
    pub password: String,
}

async fn auth_password_based(
    mut state: RequestState,
    form: Json<AuthPasswordBasedDto>,
) -> ServiceResult<AuthTokenDto> {
    let form = form.0;
    let account = state
        .db
        .get_account_by_auth_method(models::AuthRequest::PasswordBased {
            username: form.username.clone(),
        })
        .await?;

    if let Some(account) = account {
        for auth_method in account.auth_methods.iter() {
            if let models::AuthMethod::PasswordBased(password_based) = auth_method {
                if password_hash_verify(&password_based.password_hash, &form.password)?
                    && password_based.username == form.username
                {
                    let token = state
                        .db
                        .create_session_token(
                            account.id,
                            models::AuthMethodType::PasswordBased,
                            Utc::now().add(Duration::minutes(30)),
                            false,
                        )
                        .await?;

                    return Ok(AuthTokenDto { token });
                }
            }
        }
    }

    Err(ServiceError::Unauthorized("Invalid username or password"))
}

fn auth_password_based_docs(op: TransformOperation) -> TransformOperation {
    op.description("Login with username and password.")
        .tag("auth")
        .response::<200, Json<AuthTokenDto>>()
        .response_with::<401, (), _>(|res| res.description("Invalid username or password!"))
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct AuthNfcBasedNfcIdDto {
    pub card_id: String,
}

async fn auth_nfc_based_nfc_id(
    mut state: RequestState,
    form: Json<AuthNfcBasedNfcIdDto>,
) -> ServiceResult<AuthTokenDto> {
    let form = form.0;

    let card_id = general_purpose::STANDARD
        .decode(form.card_id)
        .map_err(|_| {
            ServiceError::InternalServerError(
                "Could not decode base64 parameter 'card_id'.".to_string(),
            )
        })?;

    let account = state
        .db
        .get_account_by_auth_method(models::AuthRequest::NfcBased {
            card_id: card_id.clone(),
        })
        .await?;

    if let Some(account) = account {
        for auth_method in account.auth_methods.iter() {
            if let models::AuthMethod::NfcBased(auth_nfc) = auth_method {
                if auth_nfc.card_id == card_id {
                    let token = state
                        .db
                        .create_session_token(
                            account.id,
                            models::AuthMethodType::PasswordBased,
                            Utc::now().add(Duration::minutes(30)),
                            false,
                        )
                        .await?;

                    return Ok(AuthTokenDto { token });
                }
            }
        }
    }

    Err(ServiceError::Unauthorized("Invalid card_id"))
}

fn auth_nfc_based_nfc_id_docs(op: TransformOperation) -> TransformOperation {
    op.description("Login with nfc card id.")
        .tag("auth")
        .response::<200, Json<AuthTokenDto>>()
        .response_with::<401, (), _>(|res| res.description("Invalid card_id!"))
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
#[allow(non_snake_case)]
pub struct AuthNfcBasedAsciiMifareChallengeDto {
    pub card_id: String,
    pub ek_rndB: String,
}
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
#[allow(non_snake_case)]
pub struct AuthNfcBasedAsciiMifareChallengeResponseDto {
    pub card_id: String,
    pub dk_rndA_rndBshifted: String,
}

#[allow(non_snake_case)]
async fn auth_nfc_based_ascii_mifare_challenge(
    mut state: RequestState,
    form: Json<AuthNfcBasedAsciiMifareChallengeDto>,
) -> ServiceResult<Json<AuthNfcBasedAsciiMifareChallengeResponseDto>> {
    let form = form.0;

    let card_id = general_purpose::STANDARD
        .decode(form.card_id)
        .map_err(|_| {
            ServiceError::InternalServerError(
                "Could not decode base64 parameter 'card_id'.".to_string(),
            )
        })?;

    let account = state
        .db
        .get_account_by_auth_method(models::AuthRequest::NfcBased {
            card_id: card_id.clone(),
        })
        .await?;

    if let Some(account) = account {
        let ek_rndB = general_purpose::STANDARD
            .decode(form.ek_rndB)
            .map_err(|_| {
                ServiceError::InternalServerError(
                    "Could not decode base64 parameter 'ek_rndB'.".to_string(),
                )
            })?;

        for auth_method in account.auth_methods.iter() {
            if let models::AuthMethod::NfcBased(auth_nfc) = auth_method {
                if auth_nfc.card_id == card_id {
                    let dk_rndA_rndBshifted = mifare::authenticate_nfc_mifare_desfire_phase1(
                        &state.ascii_mifare_challenge,
                        account.id,
                        auth_nfc,
                        &ek_rndB,
                    )
                    .await?;

                    return Ok(Json(AuthNfcBasedAsciiMifareChallengeResponseDto {
                        card_id: general_purpose::STANDARD.encode(card_id),
                        dk_rndA_rndBshifted: general_purpose::STANDARD.encode(dk_rndA_rndBshifted),
                    }));
                }
            }
        }
    }

    Err(ServiceError::Unauthorized("Invalid challenge!"))
}

fn auth_nfc_based_ascii_mifare_challenge_docs(op: TransformOperation) -> TransformOperation {
    op.description("Request challenge.")
        .tag("auth")
        .response::<200, Json<AuthNfcBasedAsciiMifareChallengeResponseDto>>()
        .response_with::<401, (), _>(|res| res.description("Invalid challenge!"))
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
#[allow(non_snake_case)]
pub struct AuthNfcBasedAsciiMifareResponseDto {
    pub card_id: String,
    pub dk_rndA_rndBshifted: String,
    pub ek_rndAshifted_card: String,
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct AuthNfcBasedAsciiMifareResponseResponseDto {
    pub card_id: String,
    pub session_key: String,
    pub token: String,
}

#[allow(non_snake_case)]
async fn auth_nfc_based_ascii_mifare_response(
    mut state: RequestState,
    form: Json<AuthNfcBasedAsciiMifareResponseDto>,
) -> ServiceResult<Json<AuthNfcBasedAsciiMifareResponseResponseDto>> {
    let form = form.0;

    let card_id = general_purpose::STANDARD
        .decode(form.card_id)
        .map_err(|_| {
            ServiceError::InternalServerError(
                "Could not decode base64 parameter 'card_id'.".to_string(),
            )
        })?;

    let account = state
        .db
        .get_account_by_auth_method(models::AuthRequest::NfcBased {
            card_id: card_id.clone(),
        })
        .await?;

    if let Some(account) = account {
        let dk_rndA_rndBshifted = general_purpose::STANDARD
            .decode(form.dk_rndA_rndBshifted)
            .map_err(|_| {
                ServiceError::InternalServerError(
                    "Could not decode base64 parameter 'dk_rndA_rndBshifted'.".to_string(),
                )
            })?;
        let ek_rndAshifted_card = general_purpose::STANDARD
            .decode(form.ek_rndAshifted_card)
            .map_err(|_| {
                ServiceError::InternalServerError(
                    "Could not decode base64 parameter 'ek_rndAshifted_card'.".to_string(),
                )
            })?;

        for auth_method in account.auth_methods.iter() {
            if let models::AuthMethod::NfcBased(auth_nfc) = auth_method {
                if auth_nfc.card_id == card_id {
                    let session_key = mifare::authenticate_nfc_mifare_desfire_phase2(
                        &state.ascii_mifare_challenge,
                        account.id,
                        auth_nfc,
                        &dk_rndA_rndBshifted,
                        &ek_rndAshifted_card,
                    )
                    .await?;

                    let token = state
                        .db
                        .create_session_token(
                            account.id,
                            models::AuthMethodType::PasswordBased,
                            Utc::now().add(Duration::minutes(30)),
                            false,
                        )
                        .await?;

                    return Ok(Json(AuthNfcBasedAsciiMifareResponseResponseDto {
                        card_id: general_purpose::STANDARD.encode(card_id),
                        session_key: general_purpose::STANDARD.encode(session_key),
                        token,
                    }));
                }
            }
        }
    }

    Err(ServiceError::Unauthorized("Invalid response!"))
}

fn auth_nfc_based_ascii_mifare_response_docs(op: TransformOperation) -> TransformOperation {
    op.description("Respond to challenge.")
        .tag("auth")
        .response::<200, Json<AuthNfcBasedAsciiMifareResponseResponseDto>>()
        .response_with::<401, (), _>(|res| res.description("Invalid response!"))
}

async fn auth_delete(mut state: RequestState) -> ServiceResult<StatusCode> {
    if let Some(session) = state.session {
        state.db.delete_session_token(session.token).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

fn auth_delete_docs(op: TransformOperation) -> TransformOperation {
    op.description("Logout the current session.")
        .tag("auth")
        .response_with::<204, (), _>(|res| res.description("Logout was successfull!"))
}

pub async fn auth_get_account(state: RequestState) -> ServiceResult<Json<AccountDto>> {
    let account = state.session_require_self()?;
    Ok(Json(AccountDto::from(&account)))
}

fn auth_get_account_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get an account by id.")
        .tag("accounts")
        .response::<200, Json<AccountDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested account does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}
