use crate::core::{
    transactions, Account, DbConnection, Money, Permission, Pool, Product, ServiceResult,
    Transaction,
};
use crate::identity_policy::{Action, RetrievedAccount};
use crate::login_required;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionProduct {
    pub product_id: Uuid,
    pub product: Option<Product>,
    pub amount: i32,
    pub current_price: Option<Money>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionWithProducts {
    pub transaction: Transaction,
    pub products: Vec<TransactionProduct>,
}

impl TransactionProduct {
    pub fn vec_to_transaction_product(list: Vec<(Product, i32)>) -> Vec<TransactionProduct> {
        list.into_iter()
            .map(|(p, a)| TransactionProduct {
                product_id: p.id,
                current_price: p.current_price.map(|price| price * a),
                product: Some(p),
                amount: a,
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn vec_from_transaction_product(
        conn: &DbConnection,
        list: Vec<TransactionProduct>,
    ) -> Vec<(Product, i32)> {
        list.into_iter()
            .filter_map(|p| match Product::get(&conn, &p.product_id) {
                Ok(product) => Some((product, p.amount)),
                _ => None,
            })
            .collect()
    }
}

/// GET route for `/admin/transactions/{account_id}`
pub async fn get_transactions(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars<'_>>,
    logged_account: RetrievedAccount,
    account_id: web::Path<String>,
    query: web::Query<FromToQuery>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

    let now = Local::now().naive_local();

    let from = query
        .from
        .unwrap_or_else(|| now - Duration::days(30))
        .date()
        .and_hms(0, 0, 0);
    let to = query.to.unwrap_or_else(|| now).date().and_hms(23, 59, 59);

    let list: Vec<TransactionWithProducts> =
        transactions::get_by_account(&conn, &account, &from, &to)?
            .into_iter()
            .map(|t| {
                let prods = t.get_products(&conn).unwrap_or_else(|_| Vec::new());
                let l = TransactionProduct::vec_to_transaction_product(prods);
                TransactionWithProducts {
                    transaction: t,
                    products: l,
                }
            })
            .collect();
    let list_str = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_owned());

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
        .with_data("transactions_str", &list_str)
        .render(&hb, "admin_transaction_list")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/admin/transaction/execute/{account_id}`
pub async fn post_execute_transaction(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    account_id: web::Path<String>,
    execute_form: web::Form<Execute>,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

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
            format!("/admin/transactions/{}", &account_id),
        )
        .finish())
}

/// GET route for `/admin/transaction/{account_id}/{transaction_id}`
pub async fn get_transaction_details(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars<'_>>,
    logged_account: RetrievedAccount,
    request: HttpRequest,
    path: web::Path<(String, String)>,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

    let conn = &pool.get()?;

    let account_id = Uuid::parse_str(&path.0.0)?;
    let transaction_id = Uuid::parse_str(&path.1)?;

    let account = Account::get(&conn, &account_id)?;

    let transaction = transactions::get_by_account_and_id(&conn, &account, &transaction_id)?;
    let products = transaction.get_products(&conn)?;

    let products = TransactionProduct::vec_to_transaction_product(products);

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("account", &account)
        .with_data("transaction", &transaction)
        .with_data("products", &products)
        .render(&hb, "admin_transaction_details")?;

    Ok(HttpResponse::Ok().body(body))
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

/// GET route for `/admin/transactions/generate/{account_id}/`
pub async fn get_transaction_generate_random(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars<'_>>,
    logged_account: RetrievedAccount,
    request: HttpRequest,
    path: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::ADMIN, Action::REDIRECT);

    let conn = &pool.get()?;

    let account_id = Uuid::parse_str(&path)?;

    let account = Account::get(&conn, &account_id)?;
    let now = Local::now().naive_local();

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("account", &account)
        .with_data(
            "date",
            &FromToQuery {
                from: Some((now - Duration::days(30)).date().and_hms(0, 0, 0)),
                to: Some(now.date().and_hms(23, 59, 59)),
            },
        )
        .render(&hb, "admin_transaction_generate_random")?;

    Ok(HttpResponse::Ok().body(body))
}

/// Helper to deserialize from-to queries
#[derive(Deserialize, Serialize)]
pub struct GenerateRandomQuery {
    #[serde(with = "naive_date_time_option_serializer")]
    #[serde(default = "get_none")]
    pub from: Option<NaiveDateTime>,
    #[serde(with = "naive_date_time_option_serializer")]
    #[serde(default = "get_none")]
    pub to: Option<NaiveDateTime>,
    pub avg_up: f32,
    pub avg_down: f32,
    pub count_per_day: u32,
}

/// POST route for `/admin/transactions/generate/{account_id}/`
pub async fn post_transaction_generate_random(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    path: web::Path<String>,
    data: web::Form<GenerateRandomQuery>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::ADMIN, Action::REDIRECT);

    let conn = &pool.get()?;

    let account_id = Uuid::parse_str(&path)?;

    let mut account = Account::get(&conn, &account_id)?;
    let now = Local::now().naive_local();

    let from = data
        .from
        .unwrap_or_else(|| now - Duration::days(30))
        .date()
        .and_hms(0, 0, 0);
    let to = data.to.unwrap_or_else(|| now).date().and_hms(23, 59, 59);

    transactions::generate_transactions(
        conn,
        &mut account,
        from,
        to,
        data.count_per_day,
        (data.avg_down * 100.0) as Money,
        (data.avg_up * 100.0) as Money,
    )?;

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/admin/transactions/{}", &account_id),
        )
        .finish())
}

/// GET route for `/admin/transactions/validate`
pub async fn get_transactions_validate(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::ADMIN, Action::REDIRECT);

    let conn = &pool.get()?;

    let result = transactions::validate_all(conn)?;

    Ok(HttpResponse::Ok().json(result))
}
