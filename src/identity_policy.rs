//! Implement custom IdentityPolicy that stores a session id
//! with the `CookieSessionPolicy` and the session data in the database
use actix_identity::CookieIdentityPolicy;
use actix_identity::Identity;
use actix_identity::IdentityPolicy;
use actix_web::dev::{Payload, ServiceRequest, ServiceResponse};
use actix_web::{web, Error, FromRequest, HttpRequest};
use futures::future::{err, ok, Ready};
use futures::prelude::*;

use uuid::Uuid;

use crate::core::{
    Account, DbConnection, Pool, ServiceError, ServiceResult, Session, AUTH_COOKIE_NAME,
};

// Encryption key for cookies
lazy_static::lazy_static! {
static ref SECRET_KEY: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));
}

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

/// IdentitiyPolicy that wraps the `CookieIdentityPolicy`
pub struct DbIdentityPolicy {
    cookie_policy: CookieIdentityPolicy,
}

impl DbIdentityPolicy {
    /// Create a new instance
    pub fn new() -> DbIdentityPolicy {
        let domain = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());

        DbIdentityPolicy {
            cookie_policy: CookieIdentityPolicy::new(SECRET_KEY.as_bytes())
                .name(AUTH_COOKIE_NAME)
                .path("/")
                .domain(&domain)
                .max_age_time(chrono::Duration::days(1))
                .secure(false),
        }
    }

    /// Load the string representation of a logged account from the database
    fn load_logged_account(
        &self,
        req: &mut ServiceRequest,
        session_id: String,
    ) -> ServiceResult<Option<String>> {
        let pool: web::Data<Pool> = match req.app_data() {
            Some(pool) => pool,
            None => {
                return Err(ServiceError::InternalServerError(
                    "r2d2 error",
                    "Can not extract database from request".to_owned(),
                ))
            }
        };
        let conn = &pool.get()?;

        let mut session = Session::get(&conn, &Uuid::parse_str(&session_id)?)?;

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
        let cookie_data = match self
            .cookie_policy
            .from_request(req)
            .now_or_never()
            .expect("ReadyFuture was not ready")
        {
            Ok(val) => val,
            Err(e) => return err(e),
        };
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
            session_id: session
                .id
                .to_hyphenated()
                .encode_upper(&mut Uuid::encode_buffer())
                .to_string(),
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

        let session = Session::get(&conn, &Uuid::parse_str(&self.session_id)?)?;
        session.delete(&conn)?;

        Ok(())
    }
}
