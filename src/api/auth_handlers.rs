use actix_identity::Identity;
use actix_web::{
    dev::Payload, error::BlockingError, web, Error, FromRequest, HttpRequest, HttpResponse,
};
use futures::Future;

use crate::core::{authentication_password, Account, Pool, ServiceError};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "value")]
pub enum AuthData {
    #[serde(rename = "password")]
    Password { username: String, password: String },
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

pub fn logout(id: Identity) -> HttpResponse {
    id.forget();
    HttpResponse::Ok().finish()
}

pub fn login(
    auth_data: web::Json<AuthData>,
    id: Identity,
    pool: web::Data<Pool>,
) -> impl Future<Item = HttpResponse, Error = ServiceError> {
    web::block(move || {
        let auth_data = auth_data.into_inner();
        let conn = &pool.get()?;

        match auth_data {
            AuthData::Password { username, password } => {
                authentication_password::get(conn, &username, &password)
            }
        }
    })
    .then(
        move |res: Result<Account, BlockingError<ServiceError>>| match res {
            Ok(account) => {
                let logged_account = serde_json::to_string(&LoggedAccount { id: account.id })?;
                id.remember(logged_account);
                Ok(HttpResponse::Ok().finish())
            }
            Err(err) => match err {
                BlockingError::Error(service_error) => Err(service_error),
                BlockingError::Canceled => Err(ServiceError::InternalServerError),
            },
        },
    )
}

pub fn get_me(logged_account: LoggedAccount) -> HttpResponse {
    HttpResponse::Ok().json(logged_account)
}
