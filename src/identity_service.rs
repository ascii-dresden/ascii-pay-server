use actix_http::body::{EitherBody, MessageBody};
use actix_http::Payload;
use actix_web::cookie::{self, Cookie};
use actix_web::{web, FromRequest, HttpRequest};
use futures::future::ok;
use futures::prelude::*;
use http::header;
use uuid::Uuid;

use futures_util::future::{ready, LocalBoxFuture, Ready};

use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::error::Error as StdError;
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpMessage, Result};

use crate::model::session::{
    create_longtime_session, delete_longtime_session, get_longtime_session, Session,
};
use crate::model::{Account, Permission};
use crate::utils::{env, DatabasePool, RedisPool, ServiceError, ServiceResult};

const SESSION_COOKIE_NAME: &str = "session";
const API_ACCESS_KEY_HEADER: &str = "X-Client-Cert";

lazy_static::lazy_static! {
    pub static ref SECURE_COOKIE: bool = env::BASE_URL.as_str().starts_with("https");
}

pub struct IdentityService {}

impl IdentityService {
    pub fn new() -> Self {
        Self {}
    }
}

impl<S, B> Transform<S, ServiceRequest> for IdentityService
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    B::Error: StdError,
{
    type Response = ServiceResponse<EitherBody<B>>;
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
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    B::Error: StdError,
{
    async fn on_request(req: &mut ServiceRequest) -> ServiceResult<()> {
        let app_data_database = req.app_data::<web::Data<DatabasePool>>();
        let database_pool: &web::Data<DatabasePool> = match app_data_database {
            Some(pool) => pool,
            None => {
                return Err(ServiceError::InternalServerError(
                    "app_data error",
                    "Cannot extract database from request".to_owned(),
                ))
            }
        };

        let app_data_redis = req.app_data::<web::Data<RedisPool>>();
        let redis_pool: &web::Data<RedisPool> = match app_data_redis {
            Some(pool) => pool,
            None => {
                return Err(ServiceError::InternalServerError(
                    "app_data error",
                    "Cannot extract redis from request".to_owned(),
                ))
            }
        };

        let identity_info = IdentityInfo::get(req, database_pool, redis_pool).await?;
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

impl<S, B> Service<ServiceRequest> for IdentityServiceMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    B::Error: StdError,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &self,
        cx: &mut ::core::task::Context<'_>,
    ) -> ::core::task::Poll<Result<(), Self::Error>> {
        self.service
            .poll_ready(cx)
            .map_err(::core::convert::Into::into)
    }

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let mut srv = self.service.clone();

        async move {
            match IdentityServiceMiddleware::<S>::on_request(&mut req).await {
                Ok(_) => {
                    let mut res = srv.borrow_mut().call(req).await?;

                    match IdentityServiceMiddleware::<S>::on_response(&mut res) {
                        Ok(_) => Ok(res.map_into_left_body()),
                        Err(err) => Ok(res.error_response(err).map_into_right_body()),
                    }
                }
                Err(err) => Ok(req.error_response(err).map_into_right_body()),
            }
        }
        .boxed_local()
    }
}

#[derive(Debug)]
struct IdentityInfo {
    session: Option<(Session, Account)>,
    is_cert_present: bool,
    should_write_cookie: bool,
}

impl IdentityInfo {
    async fn get(
        request: &ServiceRequest,
        database_pool: &DatabasePool,
        redis_pool: &RedisPool,
    ) -> ServiceResult<Self> {
        let is_cert_present = env::API_ACCESS_KEY.is_empty()
            || if let Some(auth_header) = request.headers().get(API_ACCESS_KEY_HEADER) {
                auth_header.to_str().unwrap_or("") == env::API_ACCESS_KEY.as_str()
            } else {
                false
            };

        // If the http authorization header contains a valid AuthToken return it
        let authorization_header = request
            .headers()
            .get(header::AUTHORIZATION)
            .map(|v| v.to_str().unwrap_or(""))
            .map(|t| t.trim_start_matches("Bearer "))
            .map(|t| Session::from(t.to_owned()));
        if let Some(session_id) = authorization_header {
            let session = if let Ok(account) =
                get_longtime_session(database_pool, redis_pool, &session_id).await
            {
                Some((session_id, account))
            } else {
                None
            };

            return Ok(Self {
                session,
                is_cert_present,
                should_write_cookie: false,
            });
        }

        // If the session cookie contains a valid AuthToken return it
        let session_cookie = request
            .cookie(SESSION_COOKIE_NAME)
            .map(|t| Session::from(t.value().to_owned()));
        if let Some(session_id) = session_cookie {
            let session = if let Ok(account) =
                get_longtime_session(database_pool, redis_pool, &session_id).await
            {
                Some((session_id, account))
            } else {
                None
            };

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
        let auth_token_query = auth_token_query.map(|t| Session::from(t.to_owned()));
        if let Some(session_id) = auth_token_query {
            let session = if let Ok(account) =
                get_longtime_session(database_pool, redis_pool, &session_id).await
            {
                Some((session_id, account))
            } else {
                None
            };

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
            let token: String = session.clone().into();

            Ok(Cookie::build(SESSION_COOKIE_NAME, token)
                .path("/")
                .domain(env::DOMAIN.as_str())
                .max_age(cookie::time::Duration::days(1))
                .secure(*SECURE_COOKIE)
                .finish())
        } else {
            Err(ServiceError::Unauthorized("session is not present"))
        }
    }
}

pub trait IdentityRequire {
    fn get_account(&self) -> ServiceResult<Option<Account>>;
    fn is_cert_present(&self) -> ServiceResult<bool>;
    fn get_auth_token(&self) -> ServiceResult<Option<String>>;

    fn require_account(&self, permission: Permission) -> ServiceResult<Account> {
        if let Some(account) = self.get_account()? {
            if account.permission >= permission {
                Ok(account)
            } else {
                Err(ServiceError::InsufficientPrivileges)
            }
        } else {
            Err(ServiceError::Unauthorized("login required"))
        }
    }

    fn require_cert(&self) -> ServiceResult<()> {
        if self.is_cert_present()? {
            Ok(())
        } else {
            Err(ServiceError::Unauthorized("client cert required"))
        }
    }

    fn require_auth_token(&self) -> ServiceResult<String> {
        if let Some(auth_token) = self.get_auth_token()? {
            Ok(auth_token)
        } else {
            Err(ServiceError::Unauthorized("cannot get auth token"))
        }
    }

    fn require_account_or_cert(&self, permission: Permission) -> ServiceResult<()> {
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
            Err(ServiceError::Unauthorized("login or client cert required"))
        }
    }
}

#[derive(Debug)]
#[allow(clippy::mutex_atomic)]
pub struct Identity {
    session: Arc<Mutex<Option<(Session, Account)>>>,
    is_cert_present: AtomicBool,
}

impl FromRequest for Identity {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(info) = req.extensions().get::<IdentityInfo>() {
            ok(Self {
                session: Arc::new(Mutex::new(info.session.as_ref().cloned())),
                is_cert_present: AtomicBool::new(info.is_cert_present),
            })
        } else {
            ok(Self {
                session: Arc::new(Mutex::new(None)),
                is_cert_present: AtomicBool::new(false),
            })
        }
    }
}

impl Identity {
    pub async fn store(
        &self,
        database_pool: &DatabasePool,
        redis_pool: &RedisPool,
        account_id: Uuid,
    ) -> ServiceResult<()> {
        let account = Account::get(database_pool, account_id).await?;
        let session = create_longtime_session(redis_pool, &account).await?;
        let mut s = self.session.lock().unwrap();
        *s = Some((session, account));
        Ok(())
    }

    pub async fn forget(&self, redis_pool: &RedisPool) -> ServiceResult<()> {
        let current_session = self.session.lock().unwrap().clone();
        if let Some((session, _)) = current_session {
            delete_longtime_session(redis_pool, &session.clone()).await?;
        }
        let mut s = self.session.lock().unwrap();
        *s = None;
        Ok(())
    }
}

impl IdentityRequire for Identity {
    fn get_account(&self) -> ServiceResult<Option<Account>> {
        Ok(self.session.lock().unwrap().as_ref().map(|t| t.1.clone()))
    }

    fn is_cert_present(&self) -> ServiceResult<bool> {
        Ok(self.is_cert_present.load(Ordering::Relaxed))
    }

    fn get_auth_token(&self) -> ServiceResult<Option<String>> {
        if let Some((s, _)) = self.session.lock().unwrap().as_ref() {
            Ok(Some(s.clone().into()))
        } else {
            Ok(None)
        }
    }
}

impl From<&grpcio::RpcContext<'_>> for Identity {
    fn from(ctx: &grpcio::RpcContext<'_>) -> Self {
        let auth_header = ctx.request_headers().iter().find(|(key, _)| {
            *key.to_ascii_lowercase() == API_ACCESS_KEY_HEADER.to_ascii_lowercase()
        });

        let is_cert_present = env::API_ACCESS_KEY.is_empty()
            || if let Some(auth_header) = auth_header {
                from_utf8(auth_header.1).unwrap_or("") == env::API_ACCESS_KEY.as_str()
            } else {
                false
            };

        Self {
            session: Arc::new(Mutex::new(None)),
            is_cert_present: AtomicBool::new(is_cert_present),
        }
    }
}

#[derive(Debug)]
pub struct IdentityMut {
    request: HttpRequest,
}

impl FromRequest for IdentityMut {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ok(Self {
            request: req.clone(),
        })
    }
}

impl IdentityMut {
    pub async fn store(
        &self,
        database_pool: &DatabasePool,
        redis_pool: &RedisPool,
        account_id: Uuid,
    ) -> ServiceResult<()> {
        let account = Account::get(database_pool, account_id).await?;
        let session = create_longtime_session(redis_pool, &account).await?;

        if let Some(info) = self.request.extensions_mut().get_mut::<IdentityInfo>() {
            info.session = Some((session, account));
            info.should_write_cookie = true;

            Ok(())
        } else {
            Err(ServiceError::Unauthorized("identify info not present"))
        }
    }

    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn forget(&self, redis_pool: &RedisPool) -> ServiceResult<()> {
        if let Some(info) = self.request.extensions_mut().get_mut::<IdentityInfo>() {
            info.session = None;
            info.should_write_cookie = true;

            if let Some((s, _)) = &info.session {
                delete_longtime_session(redis_pool, s).await?;
            }

            Ok(())
        } else {
            Err(ServiceError::Unauthorized("identify info not present"))
        }
    }
}

impl IdentityRequire for IdentityMut {
    fn get_account(&self) -> ServiceResult<Option<Account>> {
        if let Some(info) = self.request.extensions().get::<IdentityInfo>() {
            Ok(info.session.as_ref().map(|(_, a)| a.clone()))
        } else {
            Err(ServiceError::Unauthorized("identify info not present"))
        }
    }

    fn is_cert_present(&self) -> ServiceResult<bool> {
        if let Some(info) = self.request.extensions().get::<IdentityInfo>() {
            Ok(info.is_cert_present)
        } else {
            Err(ServiceError::Unauthorized("identify info not present"))
        }
    }

    fn get_auth_token(&self) -> ServiceResult<Option<String>> {
        if let Some(info) = self.request.extensions().get::<IdentityInfo>() {
            if let Some((s, _)) = &info.session {
                Ok(Some(s.clone().into()))
            } else {
                Ok(None)
            }
        } else {
            Err(ServiceError::Unauthorized("identify info not present"))
        }
    }
}
