use actix_web::{http, web, HttpResponse};
use handlebars::Handlebars;

use crate::core::{Account, Pool};
use crate::web::LoggedAccount;

pub fn list(hb: web::Data<Handlebars>, _: LoggedAccount, pool: web::Data<Pool>) -> HttpResponse {
    let conn = &pool.get().unwrap();
    let all_accounts = Account::all(&conn).unwrap();

    let body = hb.render("account_list", &all_accounts).unwrap();

    HttpResponse::Ok().body(body)
}

pub fn edit(
    hb: web::Data<Handlebars>,
    _: LoggedAccount,
    pool: web::Data<Pool>,
    account_id: web::Path<String>,
) -> HttpResponse {
    let conn = &pool.get().unwrap();
    let account = Account::get(&conn, &account_id).unwrap();

    let body = hb.render("account_edit", &account).unwrap();

    HttpResponse::Ok().body(body)
}

pub fn save(
    _: LoggedAccount,
    pool: web::Data<Pool>,
    account: web::Form<Account>,
    account_id: web::Path<String>,
) -> HttpResponse {
    println!("save");
    if *account_id != account.id {
        panic!("Oh no");
    }

    let conn = &pool.get().unwrap();
    let mut server_account = Account::get(&conn, &account_id).unwrap();

    server_account.name = account.name.empty_to_none();
    server_account.mail = account.mail.empty_to_none();
    server_account.limit = account.limit;

    server_account.update(&conn).unwrap();

    println!("redirect");
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
            Some(s) => if s.is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
    }
}
