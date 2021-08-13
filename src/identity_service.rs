use actix_http::cookie::Cookie;
use actix_http::Payload;
use actix_web::{web, FromRequest, HttpRequest};
use futures::future::ok;
use futures::prelude::*;
use http::header;
use uuid::Uuid;

use futures_util::future::{ready, LocalBoxFuture, Ready};

use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;
use std::task::{Context, Poll};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, Result,
};

use crate::core::{
    env, Account, DbConnection, Permission, Pool, ServiceError, ServiceResult, Session,
};
use crate::web::utils::{create_token_from_obj, parse_obj_from_token};

const SESSION_COOKIE_NAME: &str = "session";

lazy_static::lazy_static! {
    pub static ref SECURE_COOKIE: bool = env::BASE_URL.as_str().starts_with("https");
}

pub struct IdentityService {}

impl IdentityService {
    pub fn new() -> Self {
        Self {}
    }
}

impl<S, B> Transform<S> for IdentityService
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = IdentityServiceMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(IdentityServiceMiddleware {
            service: Rc::new(RefCell::new(service)),
        }))
    }
}

#[derive(Clone)]
pub struct IdentityServiceMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> IdentityServiceMiddleware<S>
where
    B: 'static,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    async fn on_request(req: &mut ServiceRequest) -> ServiceResult<()> {
        let app_data = req.app_data::<web::Data<Pool>>();
        let pool: &web::Data<Pool> = match app_data {
            Some(pool) => pool,
            None => {
                return Err(ServiceError::InternalServerError(
                    "r2d2 error",
                    "Cannot extract database from request".to_owned(),
                ))
            }
        };
        let conn = &pool.get()?;

        let identity_info = IdentityInfo::get(&req, &conn)?;
        req.extensions_mut().insert(identity_info);
        Ok(())
    }

    fn on_response(res: &mut ServiceResponse<B>) -> ServiceResult<()> {
        let identity_info = res.request().extensions_mut().remove::<IdentityInfo>();

        if let Some(identity_info) = identity_info {
            if identity_info.should_write_cookie {
                if identity_info.session.is_some() {
                    let cookie = identity_info.get_cookie()?;
                    res.response_mut().add_cookie(&cookie)?;
                } else {
                    res.response_mut().del_cookie(SESSION_COOKIE_NAME);
                }
            }
        }

        Ok(())
    }
}

impl<S, B> Service for IdentityServiceMiddleware<S>
where
    B: 'static,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.borrow_mut().poll_ready(cx)
    }

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        let mut srv = self.service.clone();

        async move {
            match IdentityServiceMiddleware::<S>::on_request(&mut req).await {
                Ok(_) => {
                    let mut res = srv.borrow_mut().call(req).await?;

                    match IdentityServiceMiddleware::<S>::on_response(&mut res) {
                        Ok(_) => Ok(res),
                        Err(err) => Ok(res.error_response(err)),
                    }
                }
                Err(err) => Ok(req.error_response(err)),
            }
        }
        .boxed_local()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthToken {
    session_id: Uuid,
}

impl AuthToken {
    pub fn parse_session_id_from_token(token: &str) -> ServiceResult<Uuid> {
        let auth_token: AuthToken = parse_obj_from_token(&token)?;
        Ok(auth_token.session_id)
    }

    pub fn create_token_from_session_id(session_id: &Uuid) -> ServiceResult<String> {
        let auth_token = AuthToken {
            session_id: *session_id,
        };
        create_token_from_obj(&auth_token)
    }
}

#[derive(Debug)]
struct IdentityInfo {
    session: Option<(Session, Account)>,
    is_cert_present: bool,
    should_write_cookie: bool,
}

impl IdentityInfo {
    fn get(request: &ServiceRequest, conn: &DbConnection) -> ServiceResult<Self> {
        let is_cert_present = if let Some(auth_header) = request.headers().get("X-Client-Cert") {
            auth_header.to_str().unwrap_or("") == env::API_ACCESS_KEY.as_str()
        } else {
            false
        };

        // If the http authorization header contains a valid AuthToken return it
        let authorization_header = request
            .headers()
            .get(header::AUTHORIZATION)
            .map(|v| v.to_str().unwrap_or(""))
            .map(|t| AuthToken::parse_session_id_from_token(t).ok())
            .flatten();
        if let Some(session_id) = authorization_header {
            let session = if let Ok(session) = Session::get(&conn, &session_id) {
                let account = Account::get(&conn, &session.account_id)?;
                Some((session, account))
            } else { None };

            return Ok(Self {
                session,
                is_cert_present,
                should_write_cookie: false,
            });
        }

        // If the session cookie contains a valid AuthToken return it
        let session_cookie = request
            .cookie(SESSION_COOKIE_NAME)
            .map(|c| AuthToken::parse_session_id_from_token(c.value()).ok())
            .flatten();
        if let Some(session_id) = session_cookie {
            let session = if let Ok(session) = Session::get(&conn, &session_id) {
                let account = Account::get(&conn, &session.account_id)?;
                Some((session, account))
            } else { None };

            return Ok(Self {
                session,
                is_cert_present,
                should_write_cookie: false,
            });
        }

        // If the auth token query parameter contains a valid AuthToken return it
        let mut auth_token_query: Option<&str> = None;
        for pair in request.query_string().split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if key == "auth_token" {
                    auth_token_query = Some(value);
                    break;
                }
            }
        }
        let auth_token_query = auth_token_query
            .map(|t| AuthToken::parse_session_id_from_token(t).ok())
            .flatten();
        if let Some(session_id) = auth_token_query {
            let session = if let Ok(session) = Session::get(&conn, &session_id) {
                let account = Account::get(&conn, &session.account_id)?;
                Some((session, account))
            } else { None };

            return Ok(Self {
                session,
                is_cert_present,
                should_write_cookie: false,
            });
        }

        // No AuthToken found
        Ok(Self {
            session: None,
            is_cert_present,
            should_write_cookie: false,
        })
    }

    fn get_cookie(&self) -> ServiceResult<Cookie> {
        if let Some((session, _)) = &self.session {
            let token = AuthToken::create_token_from_session_id(&session.id)?;

            Ok(Cookie::build(SESSION_COOKIE_NAME, token)
                .path("/")
                .domain(env::DOMAIN.as_str())
                .max_age(time::Duration::days(1))
                .secure(*SECURE_COOKIE)
                .finish())
        } else {
            Err(ServiceError::Unauthorized)
        }
    }
}

#[derive(Debug)]
pub struct Identity {
    request: HttpRequest,
}

impl FromRequest for Identity {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ok(Self {
            request: req.clone(),
        })
    }
}

impl Identity {
    pub fn store(&self, conn: &DbConnection, account_id: &Uuid) -> ServiceResult<()> {
        let account = Account::get(&conn, account_id)?;
        let session = Session::create_default(&conn, account_id)?;

        if let Some(info) = self.request.extensions_mut().get_mut::<IdentityInfo>() {
            info.session = Some((session, account));
            info.should_write_cookie = true;

            Ok(())
        } else {
            Err(ServiceError::Unauthorized)
        }
    }

    pub fn forget(&self, conn: &DbConnection) -> ServiceResult<()> {
        if let Some(info) = self.request.extensions_mut().get_mut::<IdentityInfo>() {
            if let Some((s, _)) = &info.session {
                s.delete(&conn)?;
            }

            info.session = None;
            info.should_write_cookie = true;

            Ok(())
        } else {
            Err(ServiceError::Unauthorized)
        }
    }

    pub fn get_account(&self) -> ServiceResult<Option<Account>> {
        if let Some(info) = self.request.extensions().get::<IdentityInfo>() {
            Ok(info.session.as_ref().map(|(_, a)| a.clone()))
        } else {
            Err(ServiceError::Unauthorized)
        }
    }

    pub fn require_account(&self, permission: Permission) -> ServiceResult<Account> {
        if let Some(account) = self.get_account()? {
            if account.permission >= permission {
                Ok(account)
            } else {
                Err(ServiceError::InsufficientPrivileges)
            }
        } else {
            Err(ServiceError::Unauthorized)
        }
    }

    pub fn require_account_with_redirect(&self, permission: Permission) -> ServiceResult<Account> {
        match self.require_account(permission) {
            Ok(account) => Ok(account),
            Err(e) => match e {
                ServiceError::Unauthorized => Err(ServiceError::Redirect("/login".to_owned())),
                _ => Err(e),
            },
        }
    }

    pub fn is_cert_present(&self) -> ServiceResult<bool> {
        if let Some(info) = self.request.extensions().get::<IdentityInfo>() {
            Ok(info.is_cert_present)
        } else {
            Err(ServiceError::Unauthorized)
        }
    }

    pub fn require_cert(&self) -> ServiceResult<()> {
        if self.is_cert_present()? {
            Ok(())
        } else {
            Err(ServiceError::Unauthorized)
        }
    }

    pub fn get_auth_token(&self) -> ServiceResult<Option<String>> {
        if let Some(info) = self.request.extensions().get::<IdentityInfo>() {
            if let Some((s, _)) = &info.session {
                Ok(Some(AuthToken::create_token_from_session_id(&s.id)?))
            } else {
                Ok(None)
            }
        } else {
            Err(ServiceError::Unauthorized)
        }
    }

    #[allow(dead_code)]
    pub fn require_auth_token(&self) -> ServiceResult<String> {
        if let Some(auth_token) = self.get_auth_token()? {
            Ok(auth_token)
        } else {
            Err(ServiceError::Unauthorized)
        }
    }

    pub fn require_account_or_cert(&self, permission: Permission) -> ServiceResult<()> {
        if self.is_cert_present()? {
            return Ok(());
        }

        if let Some(account) = self.get_account()? {
            if account.permission >= permission {
                Ok(())
            } else {
                Err(ServiceError::InsufficientPrivileges)
            }
        } else {
            Err(ServiceError::Unauthorized)
        }
    }
    #[allow(dead_code)]
    pub fn require_account_or_cert_with_redirect(
        &self,
        permission: Permission,
    ) -> ServiceResult<()> {
        match self.require_account_or_cert(permission) {
            Ok(_) => Ok(()),
            Err(e) => match e {
                ServiceError::Unauthorized => Err(ServiceError::Redirect("/login".to_owned())),
                _ => Err(e),
            },
        }
    }
}
