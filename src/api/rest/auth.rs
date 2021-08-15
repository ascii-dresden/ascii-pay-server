use actix_web::{web, HttpResponse};

use crate::{
    identity_service::{Identity, IdentityMut},
    model::{Pool, ServiceResult},
    repo::{self, LoginInput},
};

/// GET route for `/api/v1/auth`
pub async fn get_auth(identity: Identity) -> ServiceResult<HttpResponse> {
    let result = repo::get_me(&identity)?;
    Ok(HttpResponse::Ok().json(result))
}

/// POST route for `/api/v1/auth`
pub async fn post_auth(
    identity: IdentityMut,
    pool: web::Data<Pool>,
    input: web::Json<LoginInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::login_mut(conn, &identity, input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}

/// DELETE route for `/api/v1/auth`
pub async fn delete_auth(
    identity: IdentityMut,
    pool: web::Data<Pool>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::logout_mut(conn, &identity)?;
    Ok(HttpResponse::Ok().json(&result))
}
