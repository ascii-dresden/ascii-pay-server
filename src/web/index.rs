use actix_identity::Identity;
use actix_web::{dev::Payload, http, web, Error, FromRequest, HttpRequest, HttpResponse};
use handlebars::Handlebars;

use crate::core::{authentication_password, Account, Pool, ServiceError};

#[derive(Serialize, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggedAccount {
    id: String,
}

impl FromRequest for LoggedAccount {
    type Error = Error;
    type Future = Result<LoggedAccount, Error>;
    type Config = ();

    fn from_request(req: &HttpRequest, pl: &mut Payload) -> Self::Future {
        if let Some(identity) = Identity::from_request(req, pl)?.identity() {
            let account: LoggedAccount = serde_json::from_str(&identity)?;
            return Ok(account);
        }
        Err(ServiceError::Unauthorized.into())
    }
}

pub fn index(hb: web::Data<Handlebars>, logged_account: Option<LoggedAccount>, req: HttpRequest, pool: web::Data<Pool>) -> HttpResponse {
    println!("{:?}", &logged_account);
    println!("---- '{}' ----", req.query_string());

    match logged_account {
        None => {
            let data = json!({
                "error": req.query_string().contains("error")
            });
            let body = hb.render("index", &data).unwrap();

            HttpResponse::Ok().body(body)
        }
        Some(account_id) => {
            let conn = &pool.get().unwrap();
            let account = Account::get(&conn, &account_id.id).unwrap();

            let data = json!({ "name": account.name.unwrap_or(account.id) });
            let body = hb.render("home", &data).unwrap();

            HttpResponse::Ok().body(body)
        }
    }
}

pub fn login(params: web::Form<LoginForm>, id: Identity, pool: web::Data<Pool>) -> HttpResponse {
    let conn = &pool.get().unwrap();

    let login_result = authentication_password::get(conn, &params.username, &params.password);
    match login_result {
        Ok(account) => {
            let logged_account = serde_json::to_string(&LoggedAccount { id: account.id }).unwrap();
            id.remember(logged_account);

            HttpResponse::Found()
                .header(http::header::LOCATION, "/")
                .finish()
        }
        Err(_) => HttpResponse::Found()
            .header(http::header::LOCATION, "/?error")
            .finish(),
    }
}

pub fn logout(id: Identity) -> HttpResponse {
    id.forget();
    HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish()
}
