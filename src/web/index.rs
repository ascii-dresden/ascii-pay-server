use actix_identity::Identity;
use actix_web::{http, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;

use crate::core::{authentication_password, stats, Pool, ServiceResult};
use crate::web::identity_policy::LoggedAccount;
use crate::web::utils::HbData;

#[derive(Serialize, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// GET route for `/` if user is logged in
pub fn get_index(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
    request: HttpRequest
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let total = stats::get_total_balance(&conn)?;

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("total", &total)
        .render(&hb, "home")?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/login` if user is not logged in
pub fn get_login(hb: web::Data<Handlebars>, request: HttpRequest) -> ServiceResult<HttpResponse> {
    let body = HbData::new(&request)
        .with_data("error", &request.query_string().contains("error"))
        .render(&hb, "index")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/login`
pub fn post_login(
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
pub fn get_logout(
    pool: web::Data<Pool>,
    logged_account: LoggedAccount,
    id: Identity,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    logged_account.forget(conn, id)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish())
}
