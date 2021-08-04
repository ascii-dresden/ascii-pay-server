use crate::core::{transactions, wallet, Permission, Pool, ServiceResult};
use crate::identity_policy::{Action, RetrievedAccount};
use crate::login_required;
use crate::web::admin::transactions::{
    naive_date_time_option_serializer, TransactionProduct, TransactionWithProducts,
};
use crate::web::utils::HbData;
use actix_web::{web, HttpRequest, HttpResponse};
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

/// GET route for `/` if user is logged in
pub async fn get_overview(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars<'_>>,
    logged_account: RetrievedAccount,
    query: web::Query<FromToQuery>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let conn = &pool.get()?;

    let now = Local::now().naive_local();

    let from = query
        .from
        .unwrap_or_else(|| now - Duration::days(30))
        .date()
        .and_hms(0, 0, 0);
    let to = query.to.unwrap_or_else(|| now).date().and_hms(23, 59, 59);

    let list: Vec<TransactionWithProducts> =
        transactions::get_by_account(&conn, &logged_account.account, &from, &to)?
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
        .with_data("transactions", &list)
        .with_data("transactions_str", &list_str)
        .render(&hb, "default_overview")?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/transaction/{transaction_id}`
pub async fn get_transaction_details(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars<'_>>,
    logged_account: RetrievedAccount,
    request: HttpRequest,
    path: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let conn = &pool.get()?;

    let transaction_id = Uuid::parse_str(&path)?;

    let transaction =
        transactions::get_by_account_and_id(&conn, &logged_account.account, &transaction_id)?;
    let products = transaction.get_products(&conn)?;

    let products = TransactionProduct::vec_to_transaction_product(products);

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("transaction", &transaction)
        .with_data("products", &products)
        .render(&hb, "default_transaction_details")?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/AsciiPayCard.pkpass` if user is logged in
pub async fn get_apple_wallet_pass(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let conn = &pool.get()?;

    let vec = wallet::create_pass(conn, &logged_account.account)?;
    Ok(HttpResponse::Ok()
        .content_type("application/vnd.apple.pkpass")
        .body(vec))
}
