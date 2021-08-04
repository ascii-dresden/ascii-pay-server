use crate::core::{env, wallet, Account, Pool, ServiceError, ServiceResult};
use actix_web::{web, HttpRequest, HttpResponse};
use uuid::Uuid;

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
    pool: web::Data<Pool>,
    device_id: web::Path<String>,
    pass_type_id: web::Path<String>,
    serial_number: web::Path<Uuid>,
    data: web::Json<PushToken>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let authentication_token = Uuid::parse_str(
        request
            .headers()
            .get("HTTP_AUTHORIZATION")
            .map(|header_value| header_value.to_str().ok())
            .flatten()
            .unwrap_or(""),
    )?;

    let conn = &pool.get()?;

    if wallet::check_pass_authorization(conn, &serial_number, &authentication_token)? {
        if wallet::is_pass_registered_on_device(conn, &device_id, &serial_number)? {
            wallet::register_pass_on_device(
                conn,
                &device_id,
                &serial_number,
                &pass_type_id,
                &data.push_token,
            )?;
            Ok(HttpResponse::Created().finish())
        } else {
            Ok(HttpResponse::NotModified().finish())
        }
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[derive(Debug, Deserialize)]
pub struct PushToken {
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
    pool: web::Data<Pool>,
    device_id: web::Path<String>,
    pass_type_id: web::Path<String>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let passes_updated_since = request
        .match_info()
        .query("passesUpdatedSince")
        .parse::<i32>()
        .ok();

    if wallet::is_device_registered(conn, &device_id)? {
        let passes = wallet::list_passes_for_device(conn, &device_id, &pass_type_id)?;

        let updated_passes = if let Some(passes_updated_since) = passes_updated_since {
            let mut updated_passes = Vec::<Uuid>::new();

            for pass in passes {
                if wallet::get_pass_updated_at(conn, &pass)? >= passes_updated_since {
                    updated_passes.push(pass);
                }
            }

            updated_passes
        } else {
            passes
        };

        if updated_passes.is_empty() {
            Ok(HttpResponse::NoContent().finish())
        } else {
            Ok(HttpResponse::Ok().json(UpdatedPasses {
                last_updated: wallet::get_current_time(),
                serial_numbers: updated_passes,
            }))
        }
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

#[derive(Debug, Serialize)]
pub struct UpdatedPasses {
    #[serde(rename = "lastUpdated")]
    last_updated: i32,
    #[serde(rename = "serialNumbers")]
    serial_numbers: Vec<Uuid>,
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
    pool: web::Data<Pool>,
    device_id: web::Path<String>,
    serial_number: web::Path<Uuid>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let authentication_token = Uuid::parse_str(
        request
            .headers()
            .get("HTTP_AUTHORIZATION")
            .map(|header_value| header_value.to_str().ok())
            .flatten()
            .unwrap_or(""),
    )?;

    let conn = &pool.get()?;

    if wallet::check_pass_authorization(conn, &serial_number, &authentication_token)? {
        if wallet::is_pass_registered_on_device(conn, &device_id, &serial_number)? {
            wallet::unregister_pass_on_device(conn, &device_id, &serial_number)?;
            Ok(HttpResponse::Ok().finish())
        } else {
            Ok(HttpResponse::NotFound().finish())
        }
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
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
    pool: web::Data<Pool>,
    pass_type_id: web::Path<String>,
    serial_number: web::Path<Uuid>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let authentication_token = Uuid::parse_str(
        request
            .headers()
            .get("HTTP_AUTHORIZATION")
            .map(|header_value| header_value.to_str().ok())
            .flatten()
            .unwrap_or(""),
    )?;

    let conn = &pool.get()?;

    if wallet::check_pass_authorization(conn, &serial_number, &authentication_token)? {
        if pass_type_id.into_inner() != env::APPLE_WALLET_PASS_TYPE_IDENTIFIER.to_string() {
            return Err(ServiceError::NotFound);
        }

        let account = Account::get(conn, &serial_number)?;
        let vec = wallet::create_pass(conn, &account)?;
        Ok(HttpResponse::Ok()
            .content_type("application/vnd.apple.pkpass")
            .body(vec))
    } else {
        Ok(HttpResponse::Unauthorized().finish())
    }
}
