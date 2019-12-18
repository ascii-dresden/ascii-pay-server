use crate::core::{transactions, Authentication, Money, Permission, Pool, ServiceResult};
use crate::identity_policy::{Action, RetrievedAccount};
use crate::login_required;

use actix_web::{web, HttpResponse};

/// Helper to deserialize execute queries
#[derive(Deserialize)]
pub struct Execute {
    pub total: f32,
    pub authentication: Authentication,
}

/// POST route for `/api/v1/transaction/execute`
pub async fn post_execute_transaction(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    execute_data: web::Json<Execute>,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER, Action::FORBIDDEN);

    let conn = &pool.get()?;
    let mut account = execute_data.authentication.get_account(&conn)?;

    if execute_data.total != 0.0 {
        transactions::execute(
            &conn,
            &mut account,
            Some(&logged_account.account),
            (execute_data.total * 100.0) as Money,
        )?;
    }

    Ok(HttpResponse::Ok().finish())
}
