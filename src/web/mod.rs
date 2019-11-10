mod accounts;
mod index;

use actix_files as fs;
use actix_identity::Identity;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{dev::Payload, web, Error, FromRequest, HttpRequest};

use crate::core::ServiceError;

lazy_static::lazy_static! {
pub  static ref SECRET_KEY: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggedAccount {
    pub id: String,
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

pub fn init(config: &mut web::ServiceConfig) {
    let domain = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());

    config.service(
        web::scope("/")
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(SECRET_KEY.as_bytes())
                    .name("auth")
                    .path("/")
                    .domain(&domain)
                    .max_age_time(chrono::Duration::days(1))
                    .secure(false),
            ))
            .service(fs::Files::new("/stylesheets", "static/stylesheets/"))
            .service(web::resource("").route(web::get().to(index::index)))
            .service(web::resource("/login").route(web::post().to(index::login)))
            .service(web::resource("/logout").route(web::get().to(index::logout)))
            .service(web::resource("/accounts").route(web::get().to(accounts::list)))
            .service(
                web::resource("/account/create")
                    .route(web::post().to(accounts::create_post))
                    .route(web::get().to(accounts::create_get)),
            )
            .service(web::resource("/account/delete/{account_id}").route(web::get().to(accounts::delete_get)))
            .service(
                web::resource("/account/{account_id}")
                    .route(web::post().to(accounts::edit_post))
                    .route(web::get().to(accounts::edit_get)),
            )
    );
}
