use crate::core::{Permission, ServiceError, ServiceResult};
use actix_identity::Identity;
use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};

/// Represents a logged in account
#[derive(Debug, Serialize, Deserialize)]
pub struct LoggedAccount {
    pub id: String,
    pub permission: Permission,
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
    pub fn new(account_id: String, permission: Permission) -> LoggedAccount {
        LoggedAccount {
            id: account_id,
            permission,
        }
    }
    pub fn require_member(&self) -> ServiceResult<()> {
        if self.permission.is_member() || self.permission.is_admin() {
            Ok(())
        } else {
            Err(ServiceError::Unauthorized)
        }
    }
    pub fn require_admin(&self) -> ServiceResult<()> {
        if self.permission.is_admin() {
            Ok(())
        } else {
            Err(ServiceError::Unauthorized)
        }
    }
}

/// Helper to convert empty strings to `None` values
pub trait EmptyToNone<T> {
    fn empty_to_none(&self) -> Option<T>;
}

impl EmptyToNone<String> for Option<String> {
    fn empty_to_none(&self) -> Option<String> {
        match self {
            None => None,
            Some(s) => {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }
        }
    }
}

/// Helper to deserialize search queries
#[derive(Deserialize)]
pub struct Search {
    pub search: Option<String>,
}
