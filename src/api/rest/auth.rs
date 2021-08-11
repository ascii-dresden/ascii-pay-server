use crate::core::{authentication_password, Pool, ServiceResult};
use crate::identity_policy::{LoggedAccount, RetrievedAccount};
use actix_identity::Identity;
use actix_web::{web, HttpResponse};

#[derive(Serialize, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// GET route for `/api/v1/auth`
pub async fn get_auth(logged_account: RetrievedAccount) -> ServiceResult<HttpResponse> {
    match logged_account {
        RetrievedAccount::Acc(logged_account) => {
            Ok(HttpResponse::Ok().json(logged_account.account))
        }
        RetrievedAccount::Nothing => Ok(HttpResponse::Unauthorized().finish()),
    }
}

/// POST route for `/api/v1/auth`
pub async fn post_auth(
    pool: web::Data<Pool>,
    id: Identity,
    params: web::Json<LoginForm>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let login_result = authentication_password::get(conn, &params.username, &params.password);
    match login_result {
        Ok(account) => {
            LoggedAccount::new(&conn, account)?.save(id)?;

            Ok(HttpResponse::Ok().finish())
        }
        Err(_) => Ok(HttpResponse::Unauthorized().finish()),
    }
}

/// DELETE route for `/api/v1/auth`
pub async fn delete_auth(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    id: Identity,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    // TODO: Check implications of this -> any cleanup needed?
    if let RetrievedAccount::Acc(acc) = logged_account {
        acc.forget(conn, id)?;
    }

    Ok(HttpResponse::Ok().finish())
}
