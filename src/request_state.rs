use std::{collections::HashMap, sync::Arc};

use aide::OperationInput;
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    RequestPartsExt, TypedHeader,
};
use headers::{authorization::Bearer, Authorization, Cookie};
use tokio::sync::Mutex;

use crate::{
    database::{AppState, AppStateAsciiMifareChallenge, DatabaseConnection},
    error::{ServiceError, ServiceResult},
    models::{self, Session},
    SESSION_COOKIE_NAME,
};

// we can also write a custom extractor that grabs a connection from the pool
// which setup is appropriate depends on your application
pub struct RequestState {
    pub db: DatabaseConnection,
    pub session: Option<Session>,
    pub ascii_mifare_challenge: Arc<Mutex<HashMap<u64, AppStateAsciiMifareChallenge>>>,
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

        let session = if let Ok(TypedHeader(cookie)) = parts.extract::<TypedHeader<Cookie>>().await
        {
            if let Some(session_token) = cookie.get(SESSION_COOKIE_NAME) {
                db.get_session_by_session_token(session_token.to_owned())
                    .await?
            } else {
                None
            }
        } else if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            let session_token = bearer.token().to_owned();
            db.get_session_by_session_token(session_token).await?
        } else {
            None
        };

        Ok(Self {
            db,
            session,
            ascii_mifare_challenge: state.ascii_mifare_challenge.clone(),
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

    pub fn session_require_admin(&self) -> ServiceResult<()> {
        if !self.session_is_present() {
            return Err(ServiceError::Unauthorized("Missing login!"));
        }

        if self.session_is_admin() {
            return Ok(());
        }

        Err(ServiceError::Forbidden)
    }

    pub fn session_require_admin_or_self(&self, account_id: u64) -> ServiceResult<()> {
        if !self.session_is_present() {
            return Err(ServiceError::Unauthorized("Missing login!"));
        }

        if self.session_is_admin() {
            return Ok(());
        }

        if self.session_is_self(account_id) {
            return Ok(());
        }

        Err(ServiceError::Forbidden)
    }

    pub fn session_require_self(&self) -> ServiceResult<models::Account> {
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
}
