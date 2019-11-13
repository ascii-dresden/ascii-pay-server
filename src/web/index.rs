use actix_identity::Identity;
use actix_web::{http, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;

use crate::core::{authentication_password, Account, Pool, ServiceResult};
use crate::web::utils::LoggedAccount;

#[derive(Serialize, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// GET route for `/`
///
/// Show login form or dashboard
pub fn get_index(
    hb: web::Data<Handlebars>,
    logged_account: Option<LoggedAccount>,
    req: HttpRequest,
    pool: web::Data<Pool>,
) -> ServiceResult<HttpResponse> {
    match logged_account {
        None => get_index_login(hb, req),
        Some(account) => get_index_dashboard(hb, account, pool),
    }
}

/// GET route for `/` if user is not logged in
fn get_index_login(hb: web::Data<Handlebars>, req: HttpRequest) -> ServiceResult<HttpResponse> {
    let data = json!({
        "error": req.query_string().contains("error")
    });
    let body = hb.render("index", &data)?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/` if user is logged in
fn get_index_dashboard(
    hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
    pool: web::Data<Pool>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let account = Account::get(&conn, &logged_account.id);
    match account {
        Ok(account) => {
            let data = json!({ "name": account.name.unwrap_or(account.id) });
            let body = hb.render("home", &data)?;

            Ok(HttpResponse::Ok().body(body))
        }
        Err(_) => Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/logout")
            .finish()),
    }
}

/// POST route for `/`
pub fn post_index_login(
    params: web::Form<LoginForm>,
    id: Identity,
    pool: web::Data<Pool>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let login_result = authentication_password::get(conn, &params.username, &params.password);
    match login_result {
        Ok(account) => {
            let logged_account =
                serde_json::to_string(&LoggedAccount::new(account.id, account.permission))?;
            id.remember(logged_account);

            Ok(HttpResponse::Found()
                .header(http::header::LOCATION, "/")
                .finish())
        }
        Err(_) => Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/?error")
            .finish()),
    }
}

/// GET route for `/logout`
pub fn get_logout(id: Identity) -> HttpResponse {
    id.forget();
    HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish()
}
