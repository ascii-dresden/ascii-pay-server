use crate::{
    core::{authentication_password, Permission, Pool, ServiceResult},
    identity_service::Identity,
};
use actix_web::{web, HttpResponse};

#[derive(Serialize, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// GET route for `/api/v1/auth`
pub async fn get_auth(identity: Identity) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account(Permission::DEFAULT)?;
    Ok(HttpResponse::Ok().json(identity_account))
}

/// POST route for `/api/v1/auth`
pub async fn post_auth(
    identity: Identity,
    pool: web::Data<Pool>,
    params: web::Json<LoginForm>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let login_result = authentication_password::get(conn, &params.username, &params.password);
    match login_result {
        Ok(account) => {
            identity.store(&conn, &account.id)?;

            Ok(HttpResponse::Ok().finish())
        }
        Err(_) => Ok(HttpResponse::Unauthorized().finish()),
    }
}

/// DELETE route for `/api/v1/auth`
pub async fn delete_auth(identity: Identity, pool: web::Data<Pool>) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    identity.forget(&conn)?;

    Ok(HttpResponse::Ok().finish())
}
