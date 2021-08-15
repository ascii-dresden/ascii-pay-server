use crate::api::web::utils::HbData;
use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{Permission, ServiceResult};
use actix_web::{web, HttpRequest, HttpResponse};
use handlebars::Handlebars;

/// GET route for `/admin/terminal`
pub async fn get_terminal(
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::MEMBER)?;

    let body = HbData::new(&request)
        .with_account(identity_account)
        .render(&hb, "admin_terminal")?;

    Ok(HttpResponse::Ok().body(body))
}
