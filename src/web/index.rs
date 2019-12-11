use actix_identity::Identity;
use actix_web::{http, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;
use crate::login_required;
use crate::core::{authentication_password, stats, Pool, ServiceResult};
use crate::web::identity_policy::{LoggedAccount, RetrievedAccount};
use crate::web::utils::HbData;

#[derive(Serialize, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
pub struct RegisterForm {
    username: String,
    password: String,
    password2: String,
}

/// GET route for `/` if user is logged in
pub async fn get_index(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars>,
    logged_account: RetrievedAccount,
    request: HttpRequest
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account);

    let conn = &pool.get()?;

    let total = stats::get_total_balance(&conn)?;

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("total", &total)
        .render(&hb, "home")?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/login` if user is not logged in
pub async fn get_login(
    hb: web::Data<Handlebars>,
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
    hb: web::Data<Handlebars>,
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

    authentication_password::register(&conn, &account, &params.username, &params.password)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/login")
        .finish())
}
