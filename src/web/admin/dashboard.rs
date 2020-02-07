use crate::core::{Permission, Pool, ServiceResult};
use crate::identity_policy::{Action, RetrievedAccount};
use crate::login_required;
use crate::web::utils::HbData;
use actix_web::{web, HttpRequest, HttpResponse};
use handlebars::Handlebars;

/// GET route for `/admin` if user is logged in
pub async fn get_dashboard(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars<'_>>,
    logged_account: RetrievedAccount,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::ADMIN, Action::REDIRECT);

    let _conn = &pool.get()?;

    let body = HbData::new(&request)
        .with_account(logged_account)
        .render(&hb, "admin_dashboard")?;

    Ok(HttpResponse::Ok().body(body))
}
