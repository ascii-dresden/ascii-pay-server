use actix_web::{http, web, HttpResponse};
use handlebars::Handlebars;

use crate::core::{Account, Pool, ServiceResult};
use crate::web::utils::{EmptyToNone, LoggedAccount, Search};

/// GET route for `/accounts`
pub fn get_accounts(
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let mut all_accounts = Account::all(&conn)?;

    let search = if let Some(search) = &query.search {
        let lower_search = search.to_ascii_lowercase();
        all_accounts = all_accounts
            .into_iter()
            .filter(|a| {
                if let Some(name) = &a.name {
                    name.to_ascii_lowercase().contains(&lower_search)
                } else {
                    true
                }
            })
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
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
    pool: web::Data<Pool>,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let account = Account::get(&conn, &account_id)?;

    let body = hb.render("account_edit", &account)?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/account/{account_id}`
pub fn post_account_edit(
    _: LoggedAccount,
    pool: web::Data<Pool>,
    account: web::Form<Account>,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    if *account_id != account.id {
        panic!("Oh no");
    }

    let conn = &pool.get()?;
    let mut server_account = Account::get(&conn, &account_id)?;

    server_account.name = account.name.empty_to_none();
    server_account.mail = account.mail.empty_to_none();
    server_account.permission = account.permission;
    server_account.limit = account.limit;

    server_account.update(&conn)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/accounts")
        .finish())
}

/// GET route for `/account/create`
pub fn get_account_create(
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
) -> ServiceResult<HttpResponse> {
    let body = hb.render("account_create", &false)?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/account/create`
pub fn post_account_create(
    _: LoggedAccount,
    pool: web::Data<Pool>,
    account: web::Form<Account>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let mut server_account = Account::create(&conn, account.permission)?;

    server_account.name = account.name.empty_to_none();
    server_account.mail = account.mail.empty_to_none();
    server_account.limit = account.limit;

    server_account.update(&conn)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/accounts")
        .finish())
}

/// GET route for `/account/delete/{account_id}`
pub fn delete_get(
    _hb: web::Data<Handlebars>,
    _: LoggedAccount,
    _pool: web::Data<Pool>,
    _account_id: web::Path<String>,
) -> HttpResponse {
    println!("Delete is not supported!");

    HttpResponse::Found()
        .header(http::header::LOCATION, "/accounts")
        .finish()
}
