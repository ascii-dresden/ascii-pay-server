use crate::core::{Authentication, Pool, ServiceResult};
use crate::identity_policy::{LoggedAccount, RetrievedAccount};
use actix_identity::Identity;
use actix_web::{web, HttpResponse};

/// POST route for `/api/v1/login` if user is logged in
pub async fn post_login(
    pool: web::Data<Pool>,
    id: Identity,
    authentication_data: web::Json<Authentication>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let login_result = authentication_data.get_account(&conn);
    match login_result {
        Ok(account) => {
            LoggedAccount::new(&conn, account)?.save(id)?;

            Ok(HttpResponse::Ok().finish())
        }
        Err(_) => Ok(HttpResponse::Forbidden().finish()),
    }
}

/// POST route for `/api/v1/logout`
pub async fn post_logout(
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
