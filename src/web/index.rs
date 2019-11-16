use actix_identity::Identity;
use actix_web::{http, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;

use crate::core::{authentication_password, Pool, ServiceResult};
use crate::web::identity_policy::LoggedAccount;

#[derive(Serialize, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// GET route for `/` if user is not logged in
pub fn get_index_login(hb: web::Data<Handlebars>, req: HttpRequest) -> ServiceResult<HttpResponse> {
    let data = json!({
        "error": req.query_string().contains("error")
    });
    let body = hb.render("index", &data)?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/` if user is logged in
pub fn get_index_dashboard(
    _pool: web::Data<Pool>,
    hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
) -> ServiceResult<HttpResponse> {
    let data = json!({ "name": logged_account.account.name.unwrap_or(logged_account.account.id) });
    let body = hb.render("home", &data)?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/`
pub fn post_index_login(
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
            .header(http::header::LOCATION, "/?error")
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
