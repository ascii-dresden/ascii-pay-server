use crate::core::{Permission, ServiceResult};
use crate::identity_policy::{Action, RetrievedAccount};
use crate::login_required;
use crate::web::utils::HbData;
use actix_web::{web, HttpRequest, HttpResponse};
use handlebars::Handlebars;

/// GET route for `/admin/terminal`
pub async fn get_terminal(
    hb: web::Data<Handlebars<'_>>,
    logged_account: RetrievedAccount,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

    let body = HbData::new(&request)
        .with_account(logged_account)
        .render(&hb, "admin_terminal")?;

    Ok(HttpResponse::Ok().body(body))
}
