use crate::core::{transactions, Account, Money, Permission, Pool, ServiceResult};
use crate::login_required;
use crate::web::identity_policy::RetrievedAccount;
use crate::web::utils::HbData;
use actix_web::{http, web, HttpRequest, HttpResponse};
use chrono::{Duration, Local, NaiveDateTime};
use handlebars::Handlebars;

use uuid::Uuid;

/// Helper to deserialize from-to queries
#[derive(Deserialize, Serialize)]
pub struct FromToQuery {
    #[serde(with = "naive_date_time_option_serializer")]
    #[serde(default = "get_none")]
    pub from: Option<NaiveDateTime>,
    #[serde(with = "naive_date_time_option_serializer")]
    #[serde(default = "get_none")]
    pub to: Option<NaiveDateTime>,
}

fn get_none() -> Option<NaiveDateTime> {
    None
}

/// Helper to deserialize execute queries
#[derive(Deserialize)]
pub struct Execute {
    pub total: f32,
}

/// GET route for `/transactions/{account_id}`
pub async fn get_transactions(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars>,
    logged_account: RetrievedAccount,
    account_id: web::Path<String>,
    query: web::Query<FromToQuery>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER);

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

    let now = Local::now().naive_local();

    let from = query
        .from
        .unwrap_or_else(|| now - Duration::days(30))
        .date()
        .and_hms(0, 0, 0);
    let to = query.to.unwrap_or_else(|| now).date().and_hms(23, 59, 59);

    let list = transactions::get_by_account(&conn, &account, &from, &to)?;

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data(
            "date",
            &FromToQuery {
                from: Some(from),
                to: Some(to),
            },
        )
        .with_data("account", &account)
        .with_data("transactions", &list)
        .render(&hb, "transaction_list")?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/transaction/execute/{account_id}`
pub async fn post_execute_transaction(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    account_id: web::Path<String>,
    execute_form: web::Form<Execute>,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER);

    if execute_form.total != 0.0 {
        let conn = &pool.get()?;

        let mut account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

        transactions::execute(
            &conn,
            &mut account,
            Some(&logged_account.account),
            (execute_form.total * 100.0) as Money,
        )?;
    }

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/transactions/{}", &account_id),
        )
        .finish())
}

/// Serialize/Deserialize a datetime to/from only a date
pub mod naive_date_time_option_serializer {
    use chrono::{NaiveDate, NaiveDateTime};
    use serde::{de::Error, de::Visitor, Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(date: &Option<NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(d) => serializer.serialize_str(&d.format("%Y-%m-%d").to_string()),
            None => serializer.serialize_str(""),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NaiveVisitor;

        impl<'de> Visitor<'de> for NaiveVisitor {
            type Value = Option<NaiveDateTime>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("yyyy-mm-dd")
            }

            fn visit_str<E>(self, value: &str) -> Result<Option<NaiveDateTime>, E>
            where
                E: Error,
            {
                Ok(NaiveDate::parse_from_str(value, "%Y-%m-%d")
                    .map(|d| d.and_hms(0, 0, 0))
                    .ok())
            }
        }
        match deserializer.deserialize_string(NaiveVisitor) {
            Ok(x) => Ok(x),
            Err(_) => Ok(None),
        }
    }
}
