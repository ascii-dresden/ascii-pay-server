//! Module for tasks that are to be run via cronjob.
use crate::core::mail::send_report_mail;
use crate::core::{transactions, Account, DbConnection, Pool, ServiceError, ServiceResult};
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{Datelike, Local};

/// Prepares a transaction report for a given user. If the user did not do any transactions in a month, `None` is returned.
fn generate_report(
    conn: &DbConnection,
    account: &Account,
) -> ServiceResult<Option<(String, String)>> {
    // get the duration for the report
    let now = Local::today().naive_local().and_hms(0, 0, 0);
    let start = if now.month() == 1 {
        now.with_month(now.month() - 1)
            .expect("Math rules changed overnight, send help.")
            .with_day(1)
            .expect("lol")
    } else {
        // special-case the jump from jan -> dec
        now.with_year(now.year() - 1)
            .expect("Math rules changed overnight, send help.")
            .with_month(12)
            .expect("lol")
            .with_day(1)
            .expect("lol")
    };
    let end = now.with_day(1).expect("lol");

    let list = transactions::get_by_account(&conn, &account, &start, &end)?;

    let total_spent = list.iter().fold(0, |acc, ta| acc + ta.total);
    if total_spent == 0 {
        return Ok(None);
    }

    // TODO: Balance before & after, overview over transactions

    let subject_line = format!("[ascii pay] Your report for {}", start.format("%m/%Y"));
    let message = format!("Hey {user},

this is your monthly transaction report for {month} from the ascii-pay-system.

Last month, you spent {total}€.

The Ascii Pay System

----
This mail has been automatically generated. Please do not reply.
You are receiving this email because you opted in to receive monthly reports about your account activity.
If you don't want to receive these mails anymore, you can change your settings in the ascii pay system.",
        user = account.name,
        month = start.format("%B %Y"),
        total = total_spent);

    Ok(Some((subject_line, message)))
}

/// GET route for `/admin/cron/reports`
///
/// Sends account reports via mail to all users who opted in.
/// This function expects a header field "X-Cron-Auth" to be set, containing the secret defined in the `.env` file.
pub async fn send_reports(
    request: HttpRequest,
    pool: web::Data<Pool>,
) -> ServiceResult<HttpResponse> {
    // expects secret to be transmitted in Header of get request
    // verify correct secret transmission
    if let Some(auth_header) = request.headers().get("X-Cron-Auth") {
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
    for acc in accounts {
        if acc.receives_monthly_report {
            // only send mails when a report has been generated
            if let Some((subject, report)) = generate_report(&conn, &acc)? {
                send_report_mail(&acc, subject, report)?;
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}
