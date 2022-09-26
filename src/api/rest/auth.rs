use actix_web::{web, HttpResponse};
use lazy_static::__Deref;

use crate::{
    identity_service::{Identity, IdentityMut},
    repo::{self, LoginInput},
    utils::{DatabasePool, RedisPool, ServiceResult},
};

/// GET route for `/api/v1/auth`
pub async fn get_auth(
    database_pool: web::Data<DatabasePool>,
    identity: Identity,
) -> ServiceResult<HttpResponse> {
    let result = repo::get_me(database_pool.deref(), &identity).await?;
    Ok(HttpResponse::Ok().json(result))
}

/// POST route for `/api/v1/auth`
pub async fn post_auth(
    identity: IdentityMut,
    database_pool: web::Data<DatabasePool>,
    redis_pool: web::Data<RedisPool>,
    input: web::Json<LoginInput>,
) -> ServiceResult<HttpResponse> {
    let result = repo::login_mut(
        database_pool.deref(),
        redis_pool.deref(),
        &identity,
        input.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/auth`
pub async fn delete_auth(
    identity: IdentityMut,
    redis_pool: web::Data<RedisPool>,
) -> ServiceResult<HttpResponse> {
    repo::logout_mut(redis_pool.deref(), &identity).await?;
    Ok(HttpResponse::Ok().json(()))
}
