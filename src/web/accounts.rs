use actix_web::{http, web, HttpResponse};
use handlebars::Handlebars;

use crate::core::{Account, Money, Permission, Pool, Searchable, ServiceError, ServiceResult};
use crate::web::identity_policy::LoggedAccount;
use crate::web::utils::{EmptyToNone, Search};

#[derive(Debug, Serialize, Deserialize)]
pub struct FormAccount {
    pub id: String,
    pub name: String,
    pub mail: String,
    pub minimum_credit: f32,
    pub permission: Permission,
}

/// GET route for `/accounts`
pub fn get_accounts(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
    query: web::Query<Search>,
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

    let data = json!({
        "search": search,
        "accounts": all_accounts
    });

    let body = hb.render("account_list", &data)?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/account/{account_id}`
pub fn get_account_edit(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let conn = &pool.get()?;

    let account = Account::get(&conn, &account_id)?;

    let body = hb.render("account_edit", &account)?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/account/{account_id}`
pub fn post_account_edit(
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

    let mut server_account = Account::get(&conn, &account_id)?;

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
pub fn get_account_create(
    hb: web::Data<Handlebars>,
    logged_account: LoggedAccount,
) -> ServiceResult<HttpResponse> {
    logged_account.require_member()?;

    let body = hb.render("account_create", &false)?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/account/create`
pub fn post_account_create(
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
        .header(http::header::LOCATION, "/accounts")
        .finish())
}

/// GET route for `/account/delete/{account_id}`
pub fn delete_get(
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
