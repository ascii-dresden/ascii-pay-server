use crate::core::{
    authentication_barcode, authentication_nfc, authentication_password, Account, Permission, Pool,
    ServiceResult,
};
use crate::identity_policy::{Action, RetrievedAccount};
use crate::login_required;
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
    logged_account: RetrievedAccount,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let conn = &pool.get()?;

    let has_password = authentication_password::has_password(&conn, &logged_account.account)?;
    let has_qr_code =
        !authentication_barcode::get_barcodes(&conn, &logged_account.account)?.is_empty();
    let has_nfc_card = !authentication_nfc::get_nfcs(&conn, &logged_account.account)?.is_empty();
    let has_mail_address = logged_account.account.mail.is_some();
    let receives_monthly_report = logged_account.account.receives_monthly_report;

    let body = HbData::new(&request)
        .with_account(logged_account)
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
    logged_account: RetrievedAccount,
    account: web::Form<FormSettings>,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let conn = &pool.get()?;

    let mut server_account = Account::get(&conn, &logged_account.account.id)?;

    server_account.name = account.name.clone();
    let new_mail = account.mail.empty_to_none();

    // only enable monthly reports when mail address is existent
    server_account.receives_monthly_report =
        new_mail.is_some() && (account.receives_monthly_report == Some("on".to_string()));
    server_account.mail = new_mail;
    server_account.username = account.username.empty_to_none();

    server_account.update(&conn)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/settings")
        .finish())
}

/// GET route for `/settings/change-password`
pub async fn get_change_password(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
    logged_account: RetrievedAccount,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let body = HbData::new(&request)
        .with_data("error", &request.query_string().contains("error"))
        .render(&hb, "default_settings_change_password")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/settings/change-password`
pub async fn post_change_password(
    pool: web::Data<Pool>,
    params: web::Form<FormChangePassword>,
    logged_account: RetrievedAccount,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let conn = &pool.get()?;

    if !authentication_password::verify_password(
        &conn,
        &logged_account.account,
        &params.old_password,
    )? {
        return Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/settings/change-password?error")
            .finish());
    }

    if params.new_password != params.new_password2 {
        return Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/settings/change-password?error")
            .finish());
    }

    authentication_password::register(&conn, &logged_account.account, &params.new_password)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/settings")
        .finish())
}

/// GET route for `/settings/revoke-password`
pub async fn get_revoke_password(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
    logged_account: RetrievedAccount,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let body = HbData::new(&request).render(&hb, "default_settings_revoke_password")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/settings/revoke-password`
pub async fn post_revoke_password(
    pool: web::Data<Pool>,
    params: web::Form<FormRevoke>,
    logged_account: RetrievedAccount,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let conn = &pool.get()?;

    if params.revoke() {
        authentication_password::remove(&conn, &logged_account.account)?;
    }

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/settings")
        .finish())
}

/// GET route for `/settings/revoke-qr`
pub async fn get_revoke_qr(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
    logged_account: RetrievedAccount,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let body = HbData::new(&request).render(&hb, "default_settings_revoke_qr")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/settings/revoke-qr`
pub async fn post_revoke_qr(
    pool: web::Data<Pool>,
    params: web::Form<FormRevoke>,
    logged_account: RetrievedAccount,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let conn = &pool.get()?;

    if params.revoke() {
        authentication_barcode::remove(&conn, &logged_account.account)?;
    }

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/settings")
        .finish())
}

/// GET route for `/settings/revoke-nfc`
pub async fn get_revoke_nfc(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
    logged_account: RetrievedAccount,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let body = HbData::new(&request).render(&hb, "default_settings_revoke_nfc")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/settings/revoke-nfc`
pub async fn post_revoke_nfc(
    pool: web::Data<Pool>,
    params: web::Form<FormRevoke>,
    logged_account: RetrievedAccount,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::DEFAULT, Action::REDIRECT);

    let conn = &pool.get()?;

    if params.revoke() {
        authentication_nfc::remove(&conn, &logged_account.account)?;
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
