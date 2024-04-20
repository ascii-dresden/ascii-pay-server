use std::collections::HashMap;
use std::sync::Arc;

use aide::OperationInput;
use axum::extract::{FromRef, FromRequestParts, Query};
use axum::http::request::Parts;
use axum::{async_trait, RequestPartsExt};
use axum_extra::TypedHeader;
use headers::authorization::Bearer;
use headers::Authorization;
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::database::{AppState, AppStateNfcChallenge, DatabaseConnection};
use crate::error::{ServiceError, ServiceResult};
use crate::models::{self, Session};

// we can also write a custom extractor that grabs a connection from the pool
// which setup is appropriate depends on your application
pub struct RequestState {
    pub db: DatabaseConnection,
    pub session: Option<Session>,
    pub challenge_storage: Arc<Mutex<HashMap<u64, AppStateNfcChallenge>>>,
}

#[derive(Debug, Deserialize)]
struct SessionTokenQuery {
    pub session_token: String,
}
async fn get_session_token(parts: &mut Parts) -> Option<String> {
    if let Ok(TypedHeader(Authorization(bearer))) =
        parts.extract::<TypedHeader<Authorization<Bearer>>>().await
    {
        return Some(bearer.token().to_owned());
    }

    if let Ok(query) = parts.extract::<Query<SessionTokenQuery>>().await {
        return Some(query.session_token.to_owned());
    }

    None
}

#[async_trait]
impl<S> FromRequestParts<S> for RequestState
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ServiceError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);

        let connection = state
            .pool
            .acquire()
            .await
            .map_err(|err| ServiceError::InternalServerError(err.to_string()))?;
        let mut db = DatabaseConnection { connection };

        let session = if let Some(session_token) = get_session_token(parts).await {
            db.get_session_by_session_token(session_token).await?
        } else {
            None
        };

        Ok(Self {
            db,
            session,
            challenge_storage: state.ascii_mifare_challenge.clone(),
        })
    }
}

impl OperationInput for RequestState {}

impl RequestState {
    pub fn session_is_present(&self) -> bool {
        self.session.is_some()
    }

    pub fn session_is_admin(&self) -> bool {
        if let Some(ref session) = self.session {
            if matches!(session.account.role, models::Role::Admin) {
                return true;
            }
        }

        false
    }

    pub fn session_is_self(&self, account_id: u64) -> bool {
        if let Some(ref session) = self.session {
            if session.account.id == account_id {
                return true;
            }
        }

        false
    }

    pub fn session_require_admin(&self) -> ServiceResult<models::Account> {
        let account = self.session_require_login()?;

        if self.session_is_admin() {
            return Ok(account);
        }

        Err(ServiceError::Forbidden)
    }

    pub fn session_require_admin_or_self(&self, account_id: u64) -> ServiceResult<models::Account> {
        let account = self.session_require_login()?;

        if self.session_is_admin() {
            return Ok(account);
        }

        if self.session_is_self(account_id) {
            return Ok(account);
        }

        Err(ServiceError::Forbidden)
    }

    pub fn session_require_login(&self) -> ServiceResult<models::Account> {
        if !self.session_is_present() {
            return Err(ServiceError::Unauthorized("Missing login!"));
        }

        if let Some(ref session) = self.session {
            return Ok(session.account.clone());
        }

        Err(ServiceError::Forbidden)
    }

    pub fn session_require_password_reset_token(&self) -> ServiceResult<models::Account> {
        if !self.session_is_present() {
            return Err(ServiceError::Unauthorized("Missing login!"));
        }

        if let Some(ref session) = self.session {
            if matches!(
                session.auth_method,
                models::AuthMethodType::PasswordResetToken
            ) {
                return Ok(session.account.clone());
            }
        }

        Err(ServiceError::NotFound)
    }

    pub fn session_require(&self) -> ServiceResult<Session> {
        if !self.session_is_present() {
            return Err(ServiceError::Unauthorized("Missing login!"));
        }

        if let Some(ref session) = self.session {
            return Ok(session.clone());
        }

        Err(ServiceError::Forbidden)
    }
}
