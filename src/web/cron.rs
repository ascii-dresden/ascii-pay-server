//! Module for tasks that are to be run via cronjob.
use actix_web::{web, HttpRequest, HttpResponse};
use crate::core::{Pool, ServiceResult, ServiceError, Account};

/// GET route for `/admin/cron/reports`
///
/// Sends account reports via mail to all users who opted in.
pub async fn send_reports(
    request: HttpRequest,
    pool: web::Data<Pool>,
) -> ServiceResult<HttpResponse> {
    // expects secret to be transmitted in Header of get request
    // verify correct secret transmission
    if let Some(auth_header) = request.headers().get("cron_auth") {
        let cron_secret = std::env::var("CRON_SECRET").expect("CRON_SECRET must be set.");
        if cron_secret != auth_header.to_str()? {
            return Err(ServiceError::Unauthorized);
        }
    } else {
        return Err(ServiceError::Unauthorized);
    }

    let conn = &pool.get()?;
    let accounts = Account::all(conn)?;

    // assemble reports per user and send them via mail to them.
    



    Ok(HttpResponse::Ok().finish())
}
