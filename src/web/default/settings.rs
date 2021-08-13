use crate::core::{
    authentication_barcode, authentication_nfc, authentication_password, Account, Permission, Pool,
    ServiceResult,
};
use crate::identity_service::Identity;
use crate::web::utils::{EmptyToNone, HbData};
use actix_web::{http, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;

#[derive(Debug, Serialize, Deserialize)]
pub struct FormSettings {
    pub name: String,
    pub mail: String,
    pub username: String,
    pub receives_monthly_report: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormChangePassword {
    pub old_password: String,
    pub new_password: String,
    pub new_password2: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormRevoke {
    pub revoke: String,
}
impl FormRevoke {
    pub fn revoke(&self) -> bool {
        self.revoke == "on"
    }
}

/// GET route for `/settings`
pub async fn get_settings(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::DEFAULT)?;

    let conn = &pool.get()?;

    let has_password = authentication_password::has_password(&conn, &identity_account)?;
    let has_qr_code = !authentication_barcode::get_barcodes(&conn, &identity_account)?.is_empty();
    let has_nfc_card = !authentication_nfc::get_nfcs(&conn, &identity_account)?.is_empty();
    let has_mail_address = identity_account.mail.is_some();
    let receives_monthly_report = identity_account.receives_monthly_report;

    let body = HbData::new(&request)
        .with_account(identity_account)
        .with_data("has_password", &has_password)
        .with_data("has_qr_code", &has_qr_code)
        .with_data("has_nfc_card", &has_nfc_card)
        .with_data("has_mail_address", &has_mail_address)
        .with_data("receives_monthly_report", &receives_monthly_report)
        .render(&hb, "default_settings")?;

    // TODO: Checkbox is not checked although checking it works already

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/settings`
pub async fn post_settings(
    pool: web::Data<Pool>,
    identity: Identity,
    form_account: web::Form<FormSettings>,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::DEFAULT)?;

    let conn = &pool.get()?;

    let mut server_account = Account::get(&conn, &identity_account.id)?;

    server_account.name = form_account.name.clone();
    let new_mail = form_account.mail.empty_to_none();

    // only enable monthly reports when mail address is existent
    server_account.receives_monthly_report =
        new_mail.is_some() && (form_account.receives_monthly_report == Some("on".to_string()));
    server_account.mail = new_mail;
    server_account.username = form_account.username.empty_to_none();

    server_account.update(&conn)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/settings")
        .finish())
}

/// GET route for `/settings/change-password`
pub async fn get_change_password(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
    identity: Identity,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::DEFAULT)?;

    let body = HbData::new(&request)
        .with_data("error", &request.query_string().contains("error"))
        .render(&hb, "default_settings_change_password")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/settings/change-password`
pub async fn post_change_password(
    pool: web::Data<Pool>,
    params: web::Form<FormChangePassword>,
    identity: Identity,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::DEFAULT)?;

    let conn = &pool.get()?;

    if !authentication_password::verify_password(&conn, &identity_account, &params.old_password)? {
        return Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/settings/change-password?error")
            .finish());
    }

    if params.new_password != params.new_password2 {
        return Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/settings/change-password?error")
            .finish());
    }

    authentication_password::register(&conn, &identity_account, &params.new_password)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/settings")
        .finish())
}

/// GET route for `/settings/revoke-password`
pub async fn get_revoke_password(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
    identity: Identity,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::DEFAULT)?;

    let body = HbData::new(&request).render(&hb, "default_settings_revoke_password")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/settings/revoke-password`
pub async fn post_revoke_password(
    pool: web::Data<Pool>,
    params: web::Form<FormRevoke>,
    identity: Identity,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::DEFAULT)?;

    let conn = &pool.get()?;

    if params.revoke() {
        authentication_password::remove(&conn, &identity_account)?;
    }

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/settings")
        .finish())
}

/// GET route for `/settings/revoke-qr`
pub async fn get_revoke_qr(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
    identity: Identity,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::DEFAULT)?;

    let body = HbData::new(&request).render(&hb, "default_settings_revoke_qr")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/settings/revoke-qr`
pub async fn post_revoke_qr(
    pool: web::Data<Pool>,
    params: web::Form<FormRevoke>,
    identity: Identity,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::DEFAULT)?;

    let conn = &pool.get()?;

    if params.revoke() {
        authentication_barcode::remove(&conn, &identity_account)?;
    }

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/settings")
        .finish())
}

/// GET route for `/settings/revoke-nfc`
pub async fn get_revoke_nfc(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
    identity: Identity,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::DEFAULT)?;

    let body = HbData::new(&request).render(&hb, "default_settings_revoke_nfc")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/settings/revoke-nfc`
pub async fn post_revoke_nfc(
    pool: web::Data<Pool>,
    params: web::Form<FormRevoke>,
    identity: Identity,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::DEFAULT)?;

    let conn = &pool.get()?;

    if params.revoke() {
        authentication_nfc::remove(&conn, &identity_account)?;
    }

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/settings")
        .finish())
}

/// GET route for `/settings/theme/{theme}`
pub async fn get_theme(theme: web::Path<String>) -> ServiceResult<HttpResponse> {
    let expires = time::OffsetDateTime::now_utc() + time::Duration::days(365);

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/settings")
        .cookie(
            http::Cookie::build("theme", theme.into_inner())
                .path("/")
                .expires(expires)
                .finish(),
        )
        .finish())
}
