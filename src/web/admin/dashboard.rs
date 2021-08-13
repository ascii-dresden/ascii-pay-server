use crate::core::{Permission, Pool, ServiceResult};
use crate::identity_service::Identity;
use crate::web::utils::HbData;
use actix_web::{web, HttpRequest, HttpResponse};
use handlebars::Handlebars;

/// GET route for `/admin` if user is logged in
pub async fn get_dashboard(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::MEMBER)?;

    let _conn = &pool.get()?;

    let body = HbData::new(&request)
        .with_account(identity_account)
        .render(&hb, "admin_dashboard")?;

    Ok(HttpResponse::Ok().body(body))
}
