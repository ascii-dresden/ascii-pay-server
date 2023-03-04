use std::ops::Add;

use aide::axum::routing::{delete_with, get_with, post_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::http::StatusCode;
use axum::Json;
use base64::engine::general_purpose;
use base64::Engine;
use chrono::{Duration, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::models::{self, CardType};
use crate::request_state::RequestState;

use super::accounts::{AccountDto, CardTypeDto};
use super::{nfc_id, nfc_mifare, password_hash_verify};

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/auth/password",
            post_with(auth_password_based, auth_password_based_docs),
        )
        .api_route(
            "/auth/nfc/identify",
            post_with(
                auth_nfc_based_nfc_identify,
                auth_nfc_based_nfc_identify_docs,
            ),
        )
        .api_route(
            "/auth/nfc/challenge",
            post_with(auth_nfc_based_challenge, auth_nfc_based_challenge_docs),
        )
        .api_route(
            "/auth/nfc/response",
            post_with(auth_nfc_based_response, auth_nfc_based_response_docs),
        )
        .api_route(
            "/auth/nfc/simulation",
            post_with(auth_nfc_based_simulation, auth_nfc_based_simulation_docs),
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

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct AuthPasswordBasedDto {
    pub username: String,
    pub password: String,
}

async fn auth_password_based(
    mut state: RequestState,
    form: Json<AuthPasswordBasedDto>,
) -> ServiceResult<Json<AuthTokenDto>> {
    let form = form.0;

    state.db.cleanup_session_tokens().await?;

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
                            Utc::now().add(Duration::minutes(60)),
                            false,
                        )
                        .await?;

                    return Ok(Json(AuthTokenDto { token }));
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
pub struct AuthNfcBasedNfcIdentifyDto {
    pub card_id: String,
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct AuthNfcBasedNfcIdentifyResponseDto {
    pub card_id: String,
    pub card_type: Option<CardTypeDto>,
}

async fn auth_nfc_based_nfc_identify(
    mut state: RequestState,
    form: Json<AuthNfcBasedNfcIdentifyDto>,
) -> ServiceResult<Json<AuthNfcBasedNfcIdentifyResponseDto>> {
    let form = form.0;

    state.db.cleanup_session_tokens().await?;

    let card_id = general_purpose::STANDARD
        .decode(form.card_id.clone())
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
                    return Ok(Json(AuthNfcBasedNfcIdentifyResponseDto {
                        card_id: form.card_id,
                        card_type: Some((&auth_nfc.card_type).into()),
                    }));
                }
            }
        }
    }

    Ok(Json(AuthNfcBasedNfcIdentifyResponseDto {
        card_id: form.card_id,
        card_type: None,
    }))
}

fn auth_nfc_based_nfc_identify_docs(op: TransformOperation) -> TransformOperation {
    op.description("Login with nfc card id.")
        .tag("auth")
        .response::<200, Json<AuthNfcBasedNfcIdentifyResponseDto>>()
        .response_with::<401, (), _>(|res| res.description("Invalid card_id!"))
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
#[allow(non_snake_case)]
pub struct AuthNfcBasedChallengeDto {
    pub card_id: String,
    pub request: String,
}
#[derive(Debug, PartialEq, Serialize, JsonSchema)]
#[allow(non_snake_case)]
pub struct AuthNfcBasedChallengeResponseDto {
    pub card_id: String,
    pub challenge: String,
}

#[allow(non_snake_case)]
async fn auth_nfc_based_challenge(
    mut state: RequestState,
    form: Json<AuthNfcBasedChallengeDto>,
) -> ServiceResult<Json<AuthNfcBasedChallengeResponseDto>> {
    let form = form.0;

    state.db.cleanup_session_tokens().await?;

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
        let request = general_purpose::STANDARD
            .decode(form.request)
            .map_err(|_| {
                ServiceError::InternalServerError(
                    "Could not decode base64 parameter 'request'.".to_string(),
                )
            })?;

        for auth_method in account.auth_methods.iter() {
            if let models::AuthMethod::NfcBased(auth_nfc) = auth_method {
                if auth_nfc.card_id == card_id {
                    let challenge = match auth_nfc.card_type {
                        CardType::AsciiMifare => {
                            nfc_mifare::authenticate_phase_challenge(
                                &state.challenge_storage,
                                account.id,
                                auth_nfc,
                                &request,
                            )
                            .await?
                        }
                        _ => {
                            nfc_id::authenticate_phase_challenge(
                                &state.challenge_storage,
                                account.id,
                                auth_nfc,
                                &request,
                            )
                            .await?
                        }
                    };

                    return Ok(Json(AuthNfcBasedChallengeResponseDto {
                        card_id: general_purpose::STANDARD.encode(card_id),
                        challenge: general_purpose::STANDARD.encode(challenge),
                    }));
                }
            }
        }
    }

    Err(ServiceError::Unauthorized("Invalid challenge!"))
}

fn auth_nfc_based_challenge_docs(op: TransformOperation) -> TransformOperation {
    op.description("Request challenge.")
        .tag("auth")
        .response::<200, Json<AuthNfcBasedChallengeResponseDto>>()
        .response_with::<401, (), _>(|res| res.description("Invalid challenge!"))
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
#[allow(non_snake_case)]
pub struct AuthNfcBasedResponseDto {
    pub card_id: String,
    pub challenge: String,
    pub response: String,
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct AuthNfcBasedResponseResponseDto {
    pub card_id: String,
    pub token: String,
    pub session_key: String,
}

#[allow(non_snake_case)]
async fn auth_nfc_based_response(
    mut state: RequestState,
    form: Json<AuthNfcBasedResponseDto>,
) -> ServiceResult<Json<AuthNfcBasedResponseResponseDto>> {
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
        let challenge = general_purpose::STANDARD
            .decode(form.challenge)
            .map_err(|_| {
                ServiceError::InternalServerError(
                    "Could not decode base64 parameter 'challenge'.".to_string(),
                )
            })?;
        let response = general_purpose::STANDARD
            .decode(form.response)
            .map_err(|_| {
                ServiceError::InternalServerError(
                    "Could not decode base64 parameter 'response'.".to_string(),
                )
            })?;

        for auth_method in account.auth_methods.iter() {
            if let models::AuthMethod::NfcBased(auth_nfc) = auth_method {
                if auth_nfc.card_id == card_id {
                    let session_key = match auth_nfc.card_type {
                        CardType::AsciiMifare => {
                            nfc_mifare::authenticate_phase_response(
                                &state.challenge_storage,
                                account.id,
                                auth_nfc,
                                &challenge,
                                &response,
                            )
                            .await?
                        }
                        _ => {
                            nfc_id::authenticate_phase_response(
                                &state.challenge_storage,
                                account.id,
                                auth_nfc,
                                &challenge,
                                &response,
                            )
                            .await?
                        }
                    };

                    let token = state
                        .db
                        .create_session_token(
                            account.id,
                            models::AuthMethodType::NfcBased,
                            Utc::now().add(Duration::minutes(30)),
                            false,
                        )
                        .await?;

                    return Ok(Json(AuthNfcBasedResponseResponseDto {
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

fn auth_nfc_based_response_docs(op: TransformOperation) -> TransformOperation {
    op.description("Respond to challenge.")
        .tag("auth")
        .response::<200, Json<AuthNfcBasedResponseResponseDto>>()
        .response_with::<401, (), _>(|res| res.description("Invalid response!"))
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct AuthNfcBasedSimulationDto {
    pub account_id: u64,
}

async fn auth_nfc_based_simulation(
    mut state: RequestState,
    form: Json<AuthNfcBasedSimulationDto>,
) -> ServiceResult<Json<AuthTokenDto>> {
    state.db.cleanup_session_tokens().await?;

    state.session_require_admin()?;

    let form = form.0;

    let account = state.db.get_account_by_id(form.account_id).await?;

    if let Some(account) = account {
        let token = state
            .db
            .create_session_token(
                account.id,
                models::AuthMethodType::NfcBased,
                Utc::now().add(Duration::minutes(30)),
                false,
            )
            .await?;

        return Ok(Json(AuthTokenDto { token }));
    }

    Err(ServiceError::NotFound)
}

fn auth_nfc_based_simulation_docs(op: TransformOperation) -> TransformOperation {
    op.description("Simulate login with nfc card.")
        .tag("auth")
        .response::<200, Json<AuthTokenDto>>()
        .response_with::<401, (), _>(|res| res.description("Invalid card_id!"))
}

async fn auth_delete(mut state: RequestState) -> ServiceResult<StatusCode> {
    if let Some(session) = state.session {
        state.db.delete_session_token(session.token).await?;
    }

    state.db.cleanup_session_tokens().await?;

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
