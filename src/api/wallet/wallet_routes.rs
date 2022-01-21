use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{wallet, Account, Permission};
use crate::utils::{env, DatabasePool, ServiceError, ServiceResult};
use actix_web::{web, HttpRequest, HttpResponse};
use lazy_static::__Deref;
use log::info;
use uuid::Uuid;

fn get_authentication_token(request: &HttpRequest) -> Option<Uuid> {
    let header_value = request
        .headers()
        .get("Authorization")
        .map(|header_value| header_value.to_str().ok())
        .flatten()
        .unwrap_or("")
        .split(' ')
        .collect::<Vec<&str>>()
        .get(1)
        .copied()
        .unwrap_or("");

    Uuid::parse_str(header_value).ok()
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
    database_pool: web::Data<DatabasePool>,
    path: web::Path<RegisterDevicePath>,
    data: web::Json<RegisterDeviceResponse>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let authentication_token = match get_authentication_token(&request) {
        Some(token) => token,
        None => return Ok(HttpResponse::Unauthorized().finish()),
    };

    if wallet::check_pass_authorization(
        database_pool.deref(),
        path.serial_number,
        authentication_token,
    )
    .await?
    {
        if wallet::is_pass_registered_on_device(
            database_pool.deref(),
            &path.device_id,
            path.serial_number,
        )
        .await?
        {
            Ok(HttpResponse::NotModified().finish())
        } else {
            wallet::register_pass_on_device(
                database_pool.deref(),
                &path.device_id,
                path.serial_number,
                &path.pass_type_id,
                &data.push_token,
            )
            .await?;
            Ok(HttpResponse::Created().finish())
        }
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterDevicePath {
    pub device_id: String,
    pub pass_type_id: String,
    pub serial_number: Uuid,
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
    database_pool: web::Data<DatabasePool>,
    path: web::Path<UpdatePassesPath>,
    query: web::Query<UpdatePassesQuery>,
) -> ServiceResult<HttpResponse> {
    if wallet::is_device_registered(database_pool.deref(), &path.device_id).await? {
        let passes = wallet::list_passes_for_device(
            database_pool.deref(),
            &path.device_id,
            &path.pass_type_id,
        )
        .await?;

        let updated_passes = if let Some(passes_updated_since) = query.passes_updated_since {
            let mut updated_passes = Vec::<Uuid>::new();

            for pass in passes {
                if wallet::get_pass_updated_at(database_pool.deref(), pass).await?
                    > passes_updated_since
                {
                    updated_passes.push(pass);
                }
            }

            updated_passes
        } else {
            passes
        };

        let serial_numbers = updated_passes
            .iter()
            .map(|uuid| {
                uuid.to_hyphenated()
                    .encode_upper(&mut Uuid::encode_buffer())
                    .to_owned()
            })
            .collect::<Vec<String>>();

        if updated_passes.is_empty() {
            Ok(HttpResponse::NoContent().finish())
        } else {
            Ok(HttpResponse::Ok().json(UpdatedPassesResponse {
                last_updated: format!("{}", wallet::get_current_time()),
                serial_numbers,
            }))
        }
    } else {
        Ok(HttpResponse::NotFound().finish())
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
    pub passes_updated_since: Option<i32>,
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
    database_pool: web::Data<DatabasePool>,
    path: web::Path<UnregisterDevicePath>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let authentication_token = match get_authentication_token(&request) {
        Some(token) => token,
        None => return Ok(HttpResponse::Unauthorized().finish()),
    };

    if wallet::check_pass_authorization(
        database_pool.deref(),
        path.serial_number,
        authentication_token,
    )
    .await?
    {
        if wallet::is_pass_registered_on_device(
            database_pool.deref(),
            &path.device_id,
            path.serial_number,
        )
        .await?
        {
            wallet::unregister_pass_on_device(
                database_pool.deref(),
                &path.device_id,
                path.serial_number,
            )
            .await?;
            Ok(HttpResponse::Ok().finish())
        } else {
            Ok(HttpResponse::NotFound().finish())
        }
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[derive(Debug, Deserialize)]
pub struct UnregisterDevicePath {
    pub device_id: String,
    pub pass_type_id: String,
    pub serial_number: Uuid,
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
    database_pool: web::Data<DatabasePool>,
    path: web::Path<PassDeliveryPath>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let authentication_token = match get_authentication_token(&request) {
        Some(token) => token,
        None => return Ok(HttpResponse::Unauthorized().finish()),
    };

    if wallet::check_pass_authorization(
        database_pool.deref(),
        path.serial_number,
        authentication_token,
    )
    .await?
    {
        if path.pass_type_id != env::APPLE_WALLET_PASS_TYPE_IDENTIFIER.to_string() {
            return Err(ServiceError::NotFound);
        }

        let updated_at =
            wallet::get_pass_updated_at(database_pool.deref(), path.serial_number).await?;

        let last_modified = chrono::NaiveDateTime::from_timestamp(updated_at as i64, 0)
            .format("%a, %d %b %G %T GMT")
            .to_string();

        let account = Account::get(database_pool.deref(), path.serial_number).await?;
        let vec = wallet::create_pass(database_pool.deref(), &account).await?;
        Ok(HttpResponse::Ok()
            .content_type("application/vnd.apple.pkpass")
            .append_header((http::header::LAST_MODIFIED, last_modified))
            .body(vec))
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

/// GET route for `/v1/AsciiPayCard` if user is logged in
pub async fn forward_pass(identity: Identity) -> ServiceResult<HttpResponse> {
    let auth_token = identity.get_auth_token()?;

    match auth_token {
        Some(auth_token) => Ok(HttpResponse::Found()
            .append_header((
                "Location",
                format!("/v1/AsciiPayCard.pkpass?auth_token={}", auth_token),
            ))
            .finish()),
        None => Err(ServiceError::NotFound),
    }
}

/// GET route for `/v1/AsciiPayCard.pkpass` if user is logged in
pub async fn create_pass(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
) -> ServiceResult<HttpResponse> {
    let account = identity.require_account(Permission::Default)?;
    let vec = wallet::create_pass(database_pool.deref(), &account).await?;
    Ok(HttpResponse::Ok()
        .content_type("application/vnd.apple.pkpass")
        .body(vec))
}

#[derive(Debug, Deserialize)]
pub struct PassDeliveryPath {
    pub pass_type_id: String,
    pub serial_number: Uuid,
}

pub async fn log(body: web::Bytes) -> ServiceResult<HttpResponse> {
    info!("{}", std::str::from_utf8(&body).unwrap());
    Ok(HttpResponse::Ok().finish())
}
