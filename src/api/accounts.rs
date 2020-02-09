use crate::core::{
    authentication_barcode, authentication_nfc, Account, Money, Permission, Pool, ServiceError,
    ServiceResult,
};
use crate::identity_policy::{Action, RetrievedAccount};
use crate::login_required;
use crate::web::admin::accounts::{FormAccount, SearchAccount};
use crate::web::utils::{EmptyToNone, Search};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

/// GET route for `/api/v1/accounts`
pub async fn get_accounts(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER, Action::FORBIDDEN);

    let conn = &pool.get()?;

    let search = match &query.search {
        Some(s) => s.clone(),
        None => "".to_owned(),
    };

    let lower_search = search.trim().to_ascii_lowercase();
    let search_accounts: Vec<SearchAccount> = Account::all(&conn)?
        .into_iter()
        .filter_map(|p| SearchAccount::wrap(p, &lower_search))
        .collect();

    Ok(HttpResponse::Ok().json(&search_accounts))
}

/// PUT route for `/api/v1/accounts`
pub async fn put_accounts(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    account: web::Json<FormAccount>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER, Action::FORBIDDEN);

    let conn = &pool.get()?;

    let mut server_account = Account::create(&conn, &account.name, account.permission)?;

    server_account.mail = account.mail.empty_to_none();
    server_account.username = account.username.empty_to_none();
    server_account.account_number = account.account_number.empty_to_none();
    server_account.minimum_credit = (account.minimum_credit * 100.0) as Money;

    server_account.update(&conn)?;

    Ok(HttpResponse::Created().json(json!({
        "id": server_account.id
    })))
}

/// GET route for `/api/v1/account/{account_id}`
pub async fn get_account(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER, Action::FORBIDDEN);

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

    Ok(HttpResponse::Ok().json(&account))
}

/// POST route for `/api/v1/account/{account_id}`
pub async fn post_account(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    account: web::Json<FormAccount>,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER, Action::FORBIDDEN);

    if *account_id != account.id {
        return Err(ServiceError::BadRequest(
            "Id missmage",
            "The product id of the url and the form do not match!".to_owned(),
        ));
    }

    let conn = &pool.get()?;

    let mut server_account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

    server_account.name = account.name.clone();
    server_account.mail = account.mail.empty_to_none();
    server_account.username = account.username.empty_to_none();
    server_account.account_number = account.account_number.empty_to_none();
    server_account.permission = account.permission;
    server_account.minimum_credit = (account.minimum_credit * 100.0) as Money;

    server_account.update(&conn)?;

    let mut _reauth = false;

    for (key, value) in &account.extra {
        if value.trim().is_empty() {
            continue;
        }

        if key.starts_with("barcode-new") {
            authentication_barcode::register(&conn, &server_account, value).ok();
        }
        if key.starts_with("nfc-new") {
            let mut writeable = false;
            let value = if value.starts_with("ascii:") {
                writeable = true;
                value.replace("ascii:", "").trim().to_owned()
            } else {
                value.clone()
            };
            authentication_nfc::register(&conn, &server_account, &value, writeable).ok();
            _reauth = true;
        }
    }

    Ok(HttpResponse::Ok().finish())
}

/// DELETE route for `/api/v1/account/{account_id}`
pub async fn delete_account(
    _pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    _account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER, Action::FORBIDDEN);

    println!("Delete is not supported!");

    Ok(HttpResponse::MethodNotAllowed().finish())
}
