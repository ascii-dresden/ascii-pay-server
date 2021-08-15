//! Module for tasks that are to be run via cronjob.
use crate::model::mail::send_report_mail;
use crate::model::{env, transactions, Account, DbConnection, Pool, ServiceError, ServiceResult};
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{Datelike, Local};
use std::cmp::max;

fn pad_left(s: &str, width: usize) -> String {
    let mut result = String::with_capacity(width);
    result.push_str(&" ".repeat(width - s.len()));
    result.push_str(s);
    result
}
fn pad_right(s: &str, width: usize) -> String {
    let mut result = String::with_capacity(width);
    result.push_str(s);
    result.push_str(&" ".repeat(width - s.len()));
    result
}

/// Prepares a transaction report for a given user. If the user did not do any transactions in a month, `None` is returned.
fn generate_report(
    conn: &DbConnection,
    account: &Account,
) -> ServiceResult<Option<(String, String)>> {
    // get the duration for the report
    let now = Local::today().naive_local().and_hms(0, 0, 0);
    let start = if now.month() == 1 {
        // special-case the jump from jan -> dec
        now.with_year(now.year() - 1)
            .expect("Math rules changed overnight, send help.")
            .with_month(12)
            .expect("lol")
            .with_day(1)
            .expect("lol")
    } else {
        now.with_month(now.month() - 1)
            .expect("Math rules changed overnight, send help.")
            .with_day(1)
            .expect("lol")
    };
    let end = now.with_day(1).expect("lol");

    let list = transactions::get_by_account(&conn, &account, &start, &end)?;

    let total_down = list
        .iter()
        .filter(|ta| ta.total < 0)
        .fold(0, |acc, ta| acc - ta.total) as f32
        / 100.0;
    let total_up = list
        .iter()
        .filter(|ta| ta.total > 0)
        .fold(0, |acc, ta| acc + ta.total) as f32
        / 100.0;
    if total_down == 0.0 && total_up == 0.0 {
        return Ok(None);
    }

    let start_balance = list[list.len() - 1].before_credit as f32 / 100.0;
    let end_balance = list[0].after_credit as f32 / 100.0;

    let trans: Vec<(String, String, String)> = list
        .into_iter()
        .map(|ta| {
            let c1 = ta.date.format("%d.%m.%Y - %H:%M").to_string();
            let c2 = if let Ok(prods) = ta.get_products(&conn) {
                let mut prods_str = prods
                    .iter()
                    .map(|p| format!("{} x {}", p.1, p.0.name))
                    .collect::<Vec<String>>()
                    .join(", ");
                if prods_str.len() > 30 {
                    let mut help = String::with_capacity(30);
                    help.push_str(&prods_str[0..27]);
                    help.push_str("...");
                    prods_str = help;
                }
                prods_str
            } else {
                "".to_owned()
            };
            let c3 = format!("{:.2}€", ta.total as f32 / 100.0);
            (c1, c2, c3)
        })
        .collect();

    let table_head = ("Date".to_owned(), "Products".to_owned(), "Total".to_owned());

    let (w1, w2, w3) = trans.iter().fold(
        (table_head.0.len(), table_head.1.len(), table_head.2.len()),
        |(w1, w2, w3), (c1, c2, c3)| (max(c1.len(), w1), max(c2.len(), w2), max(c3.len(), w3)),
    );

    let mut table: Vec<String> = trans
        .into_iter()
        .map(|(c1, c2, c3)| {
            format!(
                " {} | {} | {}",
                pad_right(&c1, w1),
                pad_right(&c2, w2),
                pad_left(&c3, w3)
            )
        })
        .collect();

    table.insert(
        0,
        format!(
            " {} | {} | {}",
            pad_right(&table_head.0, w1),
            pad_right(&table_head.1, w2),
            pad_right(&table_head.2, w3)
        ),
    );

    table.insert(
        1,
        format!(
            "-{}-|-{}-|{}",
            "-".repeat(w1),
            "-".repeat(w2),
            "-".repeat(w3)
        ),
    );

    let table = table.join("\n");

    let subject_line = format!("[ascii pay] Your report for {}", start.format("%m/%Y"));
    let message = format!("Hey {user},

this is your monthly transaction report for {month} from the ascii pay system.

Total spent:           {total_down:5.2}€
Total charged to card: {total_up:5.2}€

Start balance: {start_balance:5.2}€
End balance:   {end_balance:5.2}€

{table}

The Ascii Pay System

----
This mail has been automatically generated. Please do not reply.
You are receiving this email because you opted in to receive monthly reports about your account activity.
If you don't want to receive these mails anymore, you can change your settings in the ascii pay system.",
        user = account.name,
        month = start.format("%B %Y"),
        total_down = total_down,
        total_up = total_up,
        start_balance = start_balance,
        end_balance = end_balance,
        table = table,
    );

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
        let cron_secret = env::CRON_SECRET.as_str();
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
