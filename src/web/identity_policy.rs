//! Implement custom IdentityPolicy that stores a session id
//! with the `CookieSessionPolicy` and the session data in the database
use actix_identity::CookieIdentityPolicy;
use actix_identity::Identity;
use actix_identity::IdentityPolicy;
use actix_web::dev::{Payload, ServiceRequest, ServiceResponse};
use actix_web::{web, Error, FromRequest, HttpRequest};

use crate::core::{
    Account, DbConnection, Permission, Pool, ServiceError, ServiceResult, Session, AUTH_COOKIE_NAME,
};

// Encryption key for cookies
lazy_static::lazy_static! {
static ref SECRET_KEY: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));
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
    type Future = Result<Option<String>, Error>;
    type ResponseFuture = Result<(), Error>;

    fn from_request(&self, req: &mut ServiceRequest) -> Self::Future {
        let cookie_data = self.cookie_policy.from_request(req)?;

        match cookie_data {
            // Some(session_id) => self.load_logged_account(req, session_id).map_err(|err| err.actix()),
            Some(session_id) => match self.load_logged_account(req, session_id) {
                Ok(s) => Ok(s),
                Err(e) => Err(e.into()),
            },
            None => Ok(None),
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
                let logged_account: LoggedAccount =
                    serde_json::from_str(&account_str).map_err(|err| {
                        let e: ServiceError = err.into();
                        e.actix()
                    })?;

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
/// Extract `LoggedAccount` from http request
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

    /// Check if the account has member or admin rights. Otherwise return `ServiceError`
    pub fn require_member(&self) -> ServiceResult<()> {
        match self.account.permission {
            Permission::ADMIN | Permission::MEMBER => Ok(()),
            _ => Err(ServiceError::Unauthorized),
        }
    }

    /// Check if the account has admin rights. Otherwise return `ServiceError`
    pub fn require_admin(&self) -> ServiceResult<()> {
        match self.account.permission {
            Permission::ADMIN => Ok(()),
            Permission::MEMBER => Err(ServiceError::InsufficientPrivileges),
            _ => Err(ServiceError::Unauthorized),
        }
    }
}
