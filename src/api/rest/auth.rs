use std::ops::DerefMut;

use actix_web::{web, HttpResponse};

use crate::{
    identity_service::{Identity, IdentityMut},
    repo::{self, LoginInput},
    utils::{DatabasePool, RedisPool, ServiceResult},
};

/// GET route for `/api/v1/auth`
pub async fn get_auth(identity: Identity) -> ServiceResult<HttpResponse> {
    let result = repo::get_me(&identity)?;
    Ok(HttpResponse::Ok().json(result))
}

/// POST route for `/api/v1/auth`
pub async fn post_auth(
    identity: IdentityMut,
    database_pool: web::Data<DatabasePool>,
    redis_pool: web::Data<RedisPool>,
    input: web::Json<LoginInput>,
) -> ServiceResult<HttpResponse> {
    let database_conn = &database_pool.get()?;
    let mut redis_conn = redis_pool.get()?;
    let result = repo::login_mut(
        database_conn,
        redis_conn.deref_mut(),
        &identity,
        input.into_inner(),
    )?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/auth`
pub async fn delete_auth(
    identity: IdentityMut,
    redis_pool: web::Data<RedisPool>,
) -> ServiceResult<HttpResponse> {
    let mut redis_conn = redis_pool.get()?;
    let result = repo::logout_mut(redis_conn.deref_mut(), &identity)?;
    Ok(HttpResponse::Ok().json(&result))
}
