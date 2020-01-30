use crate::core::{authentication_password, transactions, Permission, Pool, ServiceResult};
use crate::identity_policy::{Action, LoggedAccount, RetrievedAccount};
use crate::login_required;
use crate::web::transactions::naive_date_time_option_serializer;
use crate::web::utils::HbData;
use actix_identity::Identity;
use actix_web::{http, web, HttpRequest, HttpResponse};
use chrono::{Duration, Local, NaiveDateTime};
use handlebars::Handlebars;

#[derive(Serialize, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
pub struct RegisterForm {
    password: String,
    password2: String,
}

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
pub async fn get_index(
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

    let list = transactions::get_by_account(&conn, &logged_account.account, &from, &to)?;

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
        .render(&hb, "index")?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/login` if user is not logged in
pub async fn get_login(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let body = HbData::new(&request)
        .with_data("error", &request.query_string().contains("error"))
        .render(&hb, "login")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/login`
pub async fn post_login(
    pool: web::Data<Pool>,
    id: Identity,
    params: web::Form<LoginForm>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let login_result = authentication_password::get(conn, &params.username, &params.password);
    match login_result {
        Ok(account) => {
            LoggedAccount::new(&conn, account)?.save(id)?;

            Ok(HttpResponse::Found()
                .header(http::header::LOCATION, "/")
                .finish())
        }
        Err(_) => Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/login?error")
            .finish()),
    }
}

/// GET route for `/logout`
pub async fn get_logout(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    id: Identity,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    // TODO: Check implications of this -> any cleanup needed?
    if let RetrievedAccount::Acc(acc) = logged_account {
        acc.forget(conn, id)?;
    }

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish())
}

/// GET route for `/register/{invitation_id}` if user is not logged in
pub async fn get_register(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
    pool: web::Data<Pool>,
    invitation_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = pool.get()?;

    let _account = authentication_password::get_account_by_invitation_link(&conn, &invitation_id)?;

    let body = HbData::new(&request)
        .with_data("error", &request.query_string().contains("error"))
        .render(&hb, "register")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/register/{invitation_id}`
pub async fn post_register(
    pool: web::Data<Pool>,
    params: web::Form<RegisterForm>,
    invitation_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let account = authentication_password::get_account_by_invitation_link(&conn, &invitation_id)?;

    if params.password != params.password2 {
        return Ok(HttpResponse::Found()
            .header(
                http::header::LOCATION,
                format!("/register/{}?error", &account.id),
            )
            .finish());
    }

    authentication_password::register(&conn, &account, &params.password)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/login")
        .finish())
}
