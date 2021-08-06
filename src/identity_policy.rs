//! Implement custom IdentityPolicy that stores a session id
//! with the `CookieSessionPolicy` and the session data in the database
use actix_identity::CookieIdentityPolicy;
use actix_identity::Identity;
use actix_identity::IdentityPolicy;
use actix_web::dev::{Payload, ServiceRequest, ServiceResponse};
use actix_web::{web, Error, FromRequest, HttpRequest};
use aes::Aes128;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use futures::future::{err, ok, Ready};
use futures::prelude::*;

use crate::core::{
    env, Account, DbConnection, Pool, ServiceError, ServiceResult, Session, AUTH_COOKIE_NAME,
};

pub enum Action {
    FORBIDDEN,
    REDIRECT,
}

#[macro_export]
macro_rules! login_required {
    ($account:ident, $permission:path, $action:path) => {
        if let RetrievedAccount::Acc(acc) = $account {
            // if a logged account has been retrieved successfully, check its validity
            if acc.account.permission >= $permission {
                acc
            } else {
                return Ok(actix_web::HttpResponse::Forbidden().finish());
            }
        } else {
            // no retrieved session is equal to no session -> login
            match $action {
                Action::FORBIDDEN => {
                    return Ok(actix_web::HttpResponse::Forbidden().finish());
                }
                Action::REDIRECT => {
                    return Ok(HttpResponse::Found()
                        .header(actix_web::http::header::LOCATION, "/login")
                        .finish());
                }
            }
        }
    };
}
#[macro_export]
macro_rules! login_or_client_cert_required {
    ($request:ident, $account:ident, $permission:path, $action:path) => {
        if crate::identity_policy::is_client_cert_present($request) {
            None
        } else {
            if let RetrievedAccount::Acc(acc) = $account {
                // if a logged account has been retrieved successfully, check its validity
                if acc.account.permission >= $permission {
                    Some(acc)
                } else {
                    return Ok(actix_web::HttpResponse::Forbidden().finish());
                }
            } else {
                // no retrieved session is equal to no session -> login
                match $action {
                    Action::FORBIDDEN => {
                        return Ok(actix_web::HttpResponse::Forbidden().finish());
                    }
                    Action::REDIRECT => {
                        return Ok(HttpResponse::Found()
                            .header(actix_web::http::header::LOCATION, "/login")
                            .finish());
                    }
                }
            }
        }
    };
}

#[macro_export]
macro_rules! client_cert_required {
    ($request:ident, $action:path) => {
        if !crate::identity_policy::is_client_cert_present($request) {
            match $action {
                Action::FORBIDDEN => {
                    return Ok(actix_web::HttpResponse::Forbidden().finish());
                }
                Action::REDIRECT => {
                    return Ok(HttpResponse::Found()
                        .header(actix_web::http::header::LOCATION, "/login")
                        .finish());
                }
            }
        }
    };
}

pub fn is_client_cert_present(request: HttpRequest) -> bool {
    if let Some(auth_header) = request.headers().get("X-Client-Cert") {
        auth_header.to_str().unwrap_or("") == env::API_ACCESS_KEY.as_str()
    } else {
        false
    }
}

/// IdentitiyPolicy that wraps the `CookieIdentityPolicy`
pub struct DbIdentityPolicy {
    cookie_policy: CookieIdentityPolicy,
}

impl DbIdentityPolicy {
    /// Create a new instance
    pub fn new() -> DbIdentityPolicy {
        let secure = env::BASE_URL.as_str().starts_with("https");

        DbIdentityPolicy {
            cookie_policy: CookieIdentityPolicy::new(env::COOKIE_ENCRYPTION_KEY.as_bytes())
                .name(AUTH_COOKIE_NAME)
                .path("/")
                .domain(env::DOMAIN.as_str())
                .max_age_time(time::Duration::days(1))
                .secure(secure),
        }
    }

    /// Load the string representation of a logged account from the database
    fn load_logged_account(
        &self,
        req: &mut ServiceRequest,
        session_id: String,
    ) -> ServiceResult<Option<String>> {
        let app_data: Option<&web::Data<Pool>> = req.app_data();
        let pool: &web::Data<Pool> = match app_data {
            Some(pool) => pool,
            None => {
                return Err(ServiceError::InternalServerError(
                    "r2d2 error",
                    "Can not extract database from request".to_owned(),
                ))
            }
        };
        let conn = &pool.get()?;

        let mut session = Session::get(&conn, &session_id)?;
        let account = Account::get(conn, &session.account_id)?;

        let logged_account = LoggedAccount {
            session_id,
            account,
        };

        session.refresh();
        session.update(&conn)?;

        Ok(Some(serde_json::to_string(&logged_account)?))
    }
}

impl IdentityPolicy for DbIdentityPolicy {
    type Future = Ready<Result<Option<String>, Error>>;
    type ResponseFuture = Ready<Result<(), Error>>;

    fn from_request(&self, req: &mut ServiceRequest) -> Self::Future {
        // it's safe to unwrap this future here as it should be immediately ready
        let mut cookie_data = match self
            .cookie_policy
            .from_request(req)
            .now_or_never()
            .expect("ReadyFuture was not ready")
        {
            Ok(val) => val,
            Err(e) => return err(e),
        };

        if cookie_data.is_none() {
            for pair in req.query_string().split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    if key == "auth_token" {
                        cookie_data = auth_token_to_session_id(value).ok();
                        break;
                    }
                }
            }
        }

        match cookie_data {
            // Some(session_id) => self.load_logged_account(req, session_id).map_err(|err| err.actix()),
            Some(session_id) => match self.load_logged_account(req, session_id) {
                Ok(s) => ok(s),
                Err(ServiceError::Unauthorized) => ok(None),
                Err(e) => err(e.into()),
            },
            None => ok(None),
        }
    }

    fn to_response<B>(
        &self,
        id: Option<String>,
        changed: bool,
        res: &mut ServiceResponse<B>,
    ) -> Self::ResponseFuture {
        let id = match id {
            Some(account_str) => {
                let logged_account: LoggedAccount = match serde_json::from_str(&account_str) {
                    Ok(val) => val,
                    Err(e) => {
                        let srv_err: ServiceError = e.into();
                        return err(srv_err.actix());
                    }
                };

                Some(logged_account.session_id)
            }
            None => None,
        };
        self.cookie_policy.to_response(id, changed, res)
    }
}

/// Represents a logged in account
#[derive(Debug, Serialize, Deserialize)]
pub struct LoggedAccount {
    pub session_id: String,
    pub account: Account,
}

/// Represents an optional for a retrieved account for the middleware to return
#[derive(Debug, Serialize, Deserialize)]
pub enum RetrievedAccount {
    Acc(LoggedAccount),
    Nothing,
}

/// Extract `RetrievedAccount` from http request
impl FromRequest for RetrievedAccount {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, pl: &mut Payload) -> Self::Future {
        let request_identity = match Identity::from_request(req, pl).now_or_never().unwrap() {
            Ok(val) => val,
            Err(e) => return err(e),
        };

        if let Some(identity) = request_identity.identity() {
            let account: LoggedAccount = match serde_json::from_str(&identity) {
                Ok(val) => val,
                Err(e) => {
                    let srv_err: ServiceError = e.into();
                    return err(srv_err.actix());
                }
            };
            return ok(RetrievedAccount::Acc(account));
        }
        ok(RetrievedAccount::Nothing)
    }
}

/// Helper functions for permission check
impl LoggedAccount {
    /// Create a new logged account instance
    pub fn new(conn: &DbConnection, account: Account) -> ServiceResult<LoggedAccount> {
        let session = Session::create(&conn, &account.id)?;

        Ok(LoggedAccount {
            session_id: session.id,
            account,
        })
    }

    /// Save the logged account to the identity storage
    pub fn save(&self, id: Identity) -> ServiceResult<()> {
        let s = serde_json::to_string(self)?;
        id.remember(s);
        Ok(())
    }

    /// Delete and invalidate the current session
    pub fn forget(&self, conn: &DbConnection, id: Identity) -> ServiceResult<()> {
        id.forget();

        let session = Session::get(&conn, &self.session_id)?;
        session.delete(&conn)?;

        Ok(())
    }

    pub fn create_auth_token(&self) -> ServiceResult<String> {
        session_id_to_auth_token(&self.session_id)
    }
}

fn session_id_to_auth_token(session_id: &str) -> ServiceResult<String> {
    type Aes128Cbc = Cbc<Aes128, Pkcs7>;
    let key = hex!("000102030405060708090a0b0c0d0e0f");
    let iv = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
    let cipher = Aes128Cbc::new_from_slices(&key, &iv)?;

    let buffer = session_id.as_bytes();
    let ciphertext = cipher.encrypt_vec(&buffer);

    Ok(base64::encode(&ciphertext))
}

fn auth_token_to_session_id(auth_token: &str) -> ServiceResult<String> {
    type Aes128Cbc = Cbc<Aes128, Pkcs7>;
    let key = hex!("000102030405060708090a0b0c0d0e0f");
    let iv = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
    let cipher = Aes128Cbc::new_from_slices(&key, &iv)?;

    let ciphertext = base64::decode(auth_token)?;
    let buffer = cipher.decrypt_vec(&ciphertext)?;

    Ok(String::from_utf8(buffer)?)
}

#[cfg(test)]
mod tests {
    use crate::core::generate_uuid_str;

    use super::*;

    #[test]
    fn test_auth_token_conversion() -> ServiceResult<()> {
        let session_id = generate_uuid_str();

        let auth_token = session_id_to_auth_token(&session_id)?;
        let converted_session_id = auth_token_to_session_id(&auth_token)?;

        assert_eq!(session_id, converted_session_id);
        Ok(())
    }
}
