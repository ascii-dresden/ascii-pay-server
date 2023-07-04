use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router, TypedHeader};
use chrono::{DateTime, TimeZone, Utc};
use headers::authorization::Bearer;
use headers::{Authorization, HeaderMap, HeaderValue};
use log::info;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::ServiceResult;
use crate::models::{AppleWalletPass, AppleWalletRegistration};
use crate::request_state::RequestState;
use crate::{env, wallet};

fn get_authentication_token(request: Option<TypedHeader<Authorization<Bearer>>>) -> Option<String> {
    if let Some(TypedHeader(Authorization(bearer))) = request {
        let header_value = bearer
            .token()
            .split(' ')
            .collect::<Vec<&str>>()
            .get(1)
            .copied()
            .unwrap_or("");

        Some(String::from(header_value))
    } else {
        None
    }
}

pub fn router(app_state: AppState) -> Router {
    Router::new()
        .route(
            "/devices/{:device_id/registrations/:pass_type_id/:serial_number",
            post(register_device).delete(unregister_device),
        )
        .route(
            "/devices/:device_id/registrations/:pass_type_id",
            get(update_passes),
        )
        .route("/passes/:pass_type_id/:serial_number", get(pass_delivery))
        .route("/log", get(log))
        .route("/ApplePassCard", get(forward_pass))
        .route("/ApplePassCard.pkpass", get(create_pass))
        .with_state(app_state)
}

/// Registration
/// register a device to receive push notifications for a pass
///
/// POST /v1/devices/{device_id}/registrations/{pass_type_id}/{serial_number}
/// Header: Authorization: ApplePass <authentication_token>
/// JSON payload: { "pushToken" : <push token, which the server needs to send push notifications to this device> }
///
/// Params definition
/// :device_id      - the device's identifier
/// :pass_type_id   - the bundle identifier for a class of passes, sometimes refered to as the pass topic, e.g. pass.com.apple.backtoschoolgift, registered with WWDR
/// :serial_number  - the pass' serial number
/// :pushToken      - the value needed for Apple Push Notification service
///
/// server action: if the authentication token is correct, associate the given push token and device identifier with this pass
/// server response:
/// --> if registration succeeded: 201
/// --> if this serial number was already registered for this device: 304
/// --> if not authorized: 401
pub async fn register_device(
    mut state: RequestState,
    header: Option<TypedHeader<Authorization<Bearer>>>,
    Path(path): Path<RegisterDevicePath>,
    data: Json<RegisterDeviceResponse>,
) -> ServiceResult<StatusCode> {
    let authentication_token = match get_authentication_token(header) {
        Some(token) => token,
        None => return Ok(StatusCode::UNAUTHORIZED),
    };

    let account_id = path.serial_number.parse::<u64>()?;

    let pass = state
        .db
        .get_apple_wallet_pass(account_id, &env::APPLE_WALLET_PASS_TYPE_IDENTIFIER)
        .await?;
    let pass = if let Some(pass) = pass {
        pass
    } else {
        return Ok(StatusCode::UNAUTHORIZED);
    };

    if pass.authentication_token == authentication_token {
        let registration = state
            .db
            .get_apple_wallet_registration(
                account_id,
                &env::APPLE_WALLET_PASS_TYPE_IDENTIFIER,
                &path.device_id,
            )
            .await?;

        if registration.is_some() {
            Ok(StatusCode::NOT_MODIFIED)
        } else {
            state
                .db
                .store_apple_wallet_registration(AppleWalletRegistration {
                    account_id,
                    pass_type_id: env::APPLE_WALLET_PASS_TYPE_IDENTIFIER.to_string(),
                    device_id: path.device_id,
                    push_token: data.push_token.to_owned(),
                })
                .await?;
            Ok(StatusCode::CREATED)
        }
    } else {
        Ok(StatusCode::UNAUTHORIZED)
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterDevicePath {
    pub device_id: String,
    pub pass_type_id: String,
    pub serial_number: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterDeviceResponse {
    #[serde(rename = "pushToken")]
    push_token: String,
}

/// Updatable passes
///
/// get all serial #s associated with a device for passes that need an update
/// Optionally with a query limiter to scope the last update since
///
/// GET /v1/devices/{device_id}/registrations/{pass_type_id}
/// GET /v1/devices/{device_id}/registrations/{pass_type_id}?passesUpdatedSince=<tag>
///
/// server action: figure out which passes associated with this device have been modified since the supplied tag (if no tag provided, all associated serial #s)
/// server response:
/// --> if there are matching passes: 200, with JSON payload: { "lastUpdated" : <new tag>, "serialNumbers" : [ <array of serial #s> ] }
/// --> if there are no matching passes: 204
/// --> if unknown device identifier: 404
pub async fn update_passes(
    mut state: RequestState,
    path: Path<UpdatePassesPath>,
    query: Query<UpdatePassesQuery>,
) -> ServiceResult<Response> {
    let passes_for_device = state
        .db
        .list_passes_for_device(&env::APPLE_WALLET_PASS_TYPE_IDENTIFIER, &path.device_id)
        .await?;
    if !passes_for_device.is_empty() {
        let serial_numbers_of_passes: Vec<String> =
            if let Some(passes_updated_since) = query.passes_updated_since {
                passes_for_device
                    .iter()
                    .filter(|p| p.updated_at > passes_updated_since)
                    .map(|p| p.account_id)
                    .map(|id| id.to_string())
                    .collect()
            } else {
                passes_for_device
                    .iter()
                    .map(|p| p.account_id)
                    .map(|id| id.to_string())
                    .collect()
            };

        if serial_numbers_of_passes.is_empty() {
            Ok(StatusCode::NO_CONTENT.into_response())
        } else {
            Ok(Json(UpdatedPassesResponse {
                last_updated: format!("{}", wallet::get_current_time()),
                serial_numbers: serial_numbers_of_passes,
            })
            .into_response())
        }
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePassesPath {
    pub device_id: String,
    pub pass_type_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePassesQuery {
    #[serde(rename = "passesUpdatedSince")]
    pub passes_updated_since: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct UpdatedPassesResponse {
    #[serde(rename = "lastUpdated")]
    last_updated: String,
    #[serde(rename = "serialNumbers")]
    serial_numbers: Vec<String>,
}

/// Unregister
///
/// unregister a device to receive push notifications for a pass
///
/// DELETE /v1/devices/{device_id}/registrations/{pass_type_id}/{serial_number}
/// Header: Authorization: ApplePass {authentication_token}
///
/// server action: if the authentication token is correct, disassociate the device from this pass
/// server response:
/// --> if disassociation succeeded: 200
/// --> if not authorized: 401
pub async fn unregister_device(
    mut state: RequestState,
    path: Path<UnregisterDevicePath>,
    header: Option<TypedHeader<Authorization<Bearer>>>,
) -> ServiceResult<StatusCode> {
    let authentication_token = match get_authentication_token(header) {
        Some(token) => token,
        None => return Ok(StatusCode::UNAUTHORIZED),
    };

    let account_id = path.serial_number.parse::<u64>()?;

    let pass = state
        .db
        .get_apple_wallet_pass(account_id, &env::APPLE_WALLET_PASS_TYPE_IDENTIFIER)
        .await?;
    let pass = if let Some(pass) = pass {
        pass
    } else {
        return Ok(StatusCode::UNAUTHORIZED);
    };

    if pass.authentication_token == authentication_token {
        let registration = state
            .db
            .get_apple_wallet_registration(
                account_id,
                &env::APPLE_WALLET_PASS_TYPE_IDENTIFIER,
                &path.device_id,
            )
            .await?;

        if registration.is_some() {
            state
                .db
                .delete_apple_wallet_registration(
                    account_id,
                    &env::APPLE_WALLET_PASS_TYPE_IDENTIFIER,
                    &path.device_id,
                )
                .await?;
            Ok(StatusCode::OK)
        } else {
            Ok(StatusCode::NOT_FOUND)
        }
    } else {
        Ok(StatusCode::UNAUTHORIZED)
    }
}

#[derive(Debug, Deserialize)]
pub struct UnregisterDevicePath {
    pub device_id: String,
    pub pass_type_id: String,
    pub serial_number: String,
}

/// Pass delivery
///
/// GET /v1/passes/{pass_type_id}/{serial_number}
/// Header: Authorization: ApplePass <authentication_token>
///
/// server response:
/// --> if auth token is correct: 200, with pass data payload
/// --> if auth token is incorrect: 401
pub async fn pass_delivery(
    mut state: RequestState,
    path: Path<PassDeliveryPath>,
    header: Option<TypedHeader<Authorization<Bearer>>>,
) -> ServiceResult<Response> {
    let authentication_token = match get_authentication_token(header) {
        Some(token) => token,
        None => return Ok(StatusCode::UNAUTHORIZED.into_response()),
    };

    let account_id = path.serial_number.parse::<u64>()?;

    let pass = state
        .db
        .get_apple_wallet_pass(account_id, &env::APPLE_WALLET_PASS_TYPE_IDENTIFIER)
        .await?;
    let pass = if let Some(pass) = pass {
        pass
    } else {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    };

    if pass.authentication_token == authentication_token {
        let last_modified = Utc.timestamp_millis_opt(pass.updated_at as i64).unwrap();

        let account = state.db.get_account_by_id(account_id).await?;

        if let Some(account) = account {
            let pass_binary = wallet::create_pass_binary(&account, &pass)?;
            Ok(PassResult {
                body: pass_binary,
                last_modified: Some(last_modified),
            }
            .into_response())
        } else {
            Ok(StatusCode::UNAUTHORIZED.into_response())
        }
    } else {
        Ok(StatusCode::UNAUTHORIZED.into_response())
    }
}

/// GET route for `/v1/AsciiPayCard` if user is logged in
pub async fn forward_pass(state: RequestState) -> ServiceResult<impl IntoResponse> {
    let session = state.session_require()?;
    let uri = &format!("/v1/AsciiPayCard.pkpass?auth_token={}", session.token);

    Ok(Redirect::temporary(uri))
}

/// GET route for `/v1/AsciiPayCard.pkpass` if user is logged in
pub async fn create_pass(mut state: RequestState) -> ServiceResult<PassResult> {
    let account = state.session_require_login()?;

    let pass = state
        .db
        .get_apple_wallet_pass(account.id, &env::APPLE_WALLET_PASS_TYPE_IDENTIFIER)
        .await?;

    let pass = if let Some(pass) = pass {
        pass
    } else {
        state
            .db
            .store_apple_wallet_pass(AppleWalletPass {
                account_id: account.id,
                pass_type_id: env::APPLE_WALLET_PASS_TYPE_IDENTIFIER.to_string(),
                authentication_token: String::new(),
                qr_code: wallet::generate_random_string(64),
                updated_at: wallet::get_current_time(),
            })
            .await?
    };

    let vec = wallet::create_pass_binary(&account, &pass)?;
    Ok(PassResult {
        body: vec,
        last_modified: None,
    })
}

#[derive(Debug, Deserialize)]
pub struct PassDeliveryPath {
    pub pass_type_id: String,
    pub serial_number: String,
}

pub async fn log(body: Bytes) -> ServiceResult<Bytes> {
    info!("{}", std::str::from_utf8(&body).unwrap());
    Ok(body)
}

pub struct PassResult {
    pub body: Vec<u8>,
    pub last_modified: Option<DateTime<Utc>>,
}

impl IntoResponse for PassResult {
    fn into_response(self) -> axum::response::Response {
        let mut header = HeaderMap::new();

        if let Ok(content_type) = HeaderValue::from_str("application/vnd.apple.pkpass") {
            header.insert(header::CONTENT_TYPE, content_type);
        }

        if let Some(last_modified) = self.last_modified {
            let last_modified = last_modified.format("%a, %d %b %G %T GMT").to_string();

            if let Ok(last_modified) = HeaderValue::from_str(&last_modified) {
                header.insert(header::LAST_MODIFIED, last_modified);
            }
        }

        (StatusCode::OK, header, self.body).into_response()
    }
}
