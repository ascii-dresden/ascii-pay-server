use actix_web::{http, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;

use crate::core::{
    authentication_password, Account, Money, Permission, Pool, Searchable, ServiceError,
    ServiceResult,
};
use crate::web::identity_policy::LoggedAccount;
use crate::web::utils::{EmptyToNone, HbData, Search};

use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct FormAccount {
    pub id: String,
    pub name: String,
    pub mail: String,
    pub minimum_credit: f32,
    pub permission: Permission,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DisplayType {
    TEXT,
    LINK,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationMethod {
    pub name: String,
    pub display: Option<(DisplayType, String)>,
    pub action: Option<(String, String)>,
}

/// GET route for `/accounts`
pub async fn get_accounts(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
    query: web::Query<Search>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;

    let mut all_accounts = Account::all(&conn)?;

    let search = if let Some(search) = &query.search {
        let lower_search = search.trim().to_ascii_lowercase();
        all_accounts = all_accounts
            .into_iter()
            .filter(|a| a.contains(&lower_search))
            .collect();
        search.clone()
    } else {
        "".to_owned()
    };

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("search", &search)
        .with_data("accounts", &all_accounts)
        .render(&hb, "account_list")?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/account/{account_id}`
pub async fn get_account_edit(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
    account_id: web::Path<String>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

    let mut authentication_methods: Vec<AuthenticationMethod> = vec![];

    if let Some(invitation_link) = authentication_password::get_invitation_link(&conn, &account)? {
        authentication_methods.push(AuthenticationMethod {
            name: "Invite link".to_owned(),
            display: Some((DisplayType::LINK, format!("/register/{}", invitation_link))),
            action: Some((
                "Revoke".to_owned(),
                format!("/account/revoke/{}", &account.id),
            )),
        });
    }
    for username in authentication_password::get_usernames(&conn, &account)? {
        authentication_methods.push(AuthenticationMethod {
            name: "Username".to_owned(),
            display: Some((DisplayType::TEXT, username)),
            action: Some((
                "Revoke".to_owned(),
                format!("/account/revoke/{}", &account.id),
            )),
        });
    }
    if authentication_methods.is_empty() {
        authentication_methods.push(AuthenticationMethod {
            name: "Password".to_owned(),
            display: None,
            action: Some((
                "Create invitation".to_owned(),
                format!("/account/invite/{}", &account.id),
            )),
        });
    }

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("account", &account)
        .with_data("authentication_methods", &authentication_methods)
        .render(&hb, "account_edit")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/account/{account_id}`
pub async fn post_account_edit(
    pool: web::Data<Pool>,
    logged_account: LoggedAccount,
    account: web::Form<FormAccount>,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    if *account_id != account.id {
        return Err(ServiceError::BadRequest(
            "Id missmage",
            "The product id of the url and the form do not match!".to_owned(),
        ));
    }

    let conn = &pool.get()?;

    let mut server_account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

    server_account.name = account.name.empty_to_none();
    server_account.mail = account.mail.empty_to_none();
    server_account.permission = account.permission;
    server_account.minimum_credit = (account.minimum_credit * 100.0) as Money;

    server_account.update(&conn)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/accounts")
        .finish())
}

/// GET route for `/account/create`
pub async fn get_account_create(
    hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let body = HbData::new(&request)
        .with_account(logged_account)
        .render(&hb, "account_create")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/account/create`
pub async fn post_account_create(
    pool: web::Data<Pool>,
    logged_account: LoggedAccount,
    account: web::Form<FormAccount>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;

    let mut server_account = Account::create(&conn, account.permission)?;

    server_account.name = account.name.empty_to_none();
    server_account.mail = account.mail.empty_to_none();
    server_account.minimum_credit = (account.minimum_credit * 100.0) as Money;

    server_account.update(&conn)?;

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/account/{}", server_account.id),
        )
        .finish())
}

/// GET route for `/account/invite/{account_id}`
pub async fn invite_get(
    pool: web::Data<Pool>,
    logged_account: LoggedAccount,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;
    authentication_password::create_invitation_link(&conn, &account)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, format!("/account/{}", account.id))
        .finish())
}

/// GET route for `/account/revoke/{account_id}`
pub async fn revoke_get(
    pool: web::Data<Pool>,
    logged_account: LoggedAccount,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;
    authentication_password::revoke_invitation_link(&conn, &account)?;
    authentication_password::remove(&conn, &account)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, format!("/account/{}", account.id))
        .finish())
}

/// GET route for `/account/delete/{account_id}`
pub async fn delete_get(
    _hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
    _account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    println!("Delete is not supported!");

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/accounts")
        .finish())
}
