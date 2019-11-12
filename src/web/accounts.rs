use actix_web::{http, web, HttpResponse};
use handlebars::Handlebars;

use crate::core::{Account, Pool};
use crate::web::{LoggedAccount, WebResult};

#[derive(Deserialize)]
pub struct Search {
    search: Option<String>,
}

pub fn list(
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> WebResult<HttpResponse> {
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

pub fn edit_get(
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
    pool: web::Data<Pool>,
    account_id: web::Path<String>,
) -> WebResult<HttpResponse> {
    let conn = &pool.get()?;
    let account = Account::get(&conn, &account_id)?;

    let body = hb.render("account_edit", &account)?;

    Ok(HttpResponse::Ok().body(body))
}

pub fn edit_post(
    _: LoggedAccount,
    pool: web::Data<Pool>,
    account: web::Form<Account>,
    account_id: web::Path<String>,
) -> WebResult<HttpResponse> {
    if *account_id != account.id {
        panic!("Oh no");
    }

    let conn = &pool.get()?;
    let mut server_account = Account::get(&conn, &account_id)?;

    server_account.name = account.name.empty_to_none();
    server_account.mail = account.mail.empty_to_none();
    server_account.limit = account.limit;

    server_account.update(&conn)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/accounts")
        .finish())
}

pub fn create_get(hb: web::Data<Handlebars>, _: LoggedAccount) -> WebResult<HttpResponse> {
    let body = hb.render("account_create", &false)?;

    Ok(HttpResponse::Ok().body(body))
}

pub fn create_post(
    _: LoggedAccount,
    pool: web::Data<Pool>,
    account: web::Form<Account>,
) -> WebResult<HttpResponse> {
    let conn = &pool.get()?;

    let mut server_account = Account::create(&conn)?;

    server_account.name = account.name.empty_to_none();
    server_account.mail = account.mail.empty_to_none();
    server_account.limit = account.limit;

    server_account.update(&conn)?;

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/accounts")
        .finish())
}

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

trait EmptyToNone<T> {
    fn empty_to_none(&self) -> Option<T>;
}

impl EmptyToNone<String> for Option<String> {
    fn empty_to_none(&self) -> Option<String> {
        match self {
            None => None,
            Some(s) => {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }
        }
    }
}
