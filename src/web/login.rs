use crate::core::{authentication_password, Pool, ServiceResult};
use crate::identity_policy::{LoggedAccount, RetrievedAccount};
use crate::web::utils::HbData;
use actix_identity::Identity;
use actix_web::{http, web, HttpRequest, HttpResponse};
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

    let account = authentication_password::get_account_by_invitation_link(&conn, &invitation_id)?;

    let body = HbData::new(&request)
        .with_data("error", &request.query_string().contains("error"))
        .with_data("account", &account)
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
