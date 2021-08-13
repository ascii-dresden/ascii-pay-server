use crate::core::{
    authentication_barcode, authentication_nfc, Account, Permission, Pool, ServiceError,
    ServiceResult,
};
use crate::identity_service::Identity;
use crate::web::admin::accounts::SearchAccount;
use crate::web::utils::Search;
use actix_web::{web, HttpResponse};
use uuid::Uuid;

/// GET route for `/api/v1/accounts`
pub async fn get_accounts(
    pool: web::Data<Pool>,
    identity: Identity,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

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
    identity: Identity,
    account: web::Json<Account>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let mut server_account = Account::create(&conn, &account.name, account.permission)?;

    server_account.minimum_credit = account.minimum_credit;
    server_account.name = account.name.clone();
    server_account.mail = account.mail.clone();
    server_account.username = account.username.clone();
    server_account.account_number = account.account_number.clone();
    server_account.permission = account.permission;

    server_account.update(&conn)?;

    Ok(HttpResponse::Created().json(json!({
        "id": server_account.id
    })))
}

/// GET route for `/api/v1/account/{account_id}`
pub async fn get_account(
    pool: web::Data<Pool>,
    identity: Identity,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

    Ok(HttpResponse::Ok().json(&account))
}

/// POST route for `/api/v1/account/{account_id}`
pub async fn post_account(
    pool: web::Data<Pool>,
    identity: Identity,
    account: web::Json<Account>,
    account_id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    if *account_id != account.id {
        return Err(ServiceError::BadRequest(
            "Id missmage",
            "The account id of the url and the json do not match!".to_owned(),
        ));
    }

    let conn = &pool.get()?;

    let mut server_account = Account::get(&conn, &account_id)?;

    server_account.minimum_credit = account.minimum_credit;
    server_account.name = account.name.clone();
    server_account.mail = account.mail.clone();
    server_account.username = account.username.clone();
    server_account.account_number = account.account_number.clone();
    server_account.permission = account.permission;

    server_account.update(&conn)?;

    Ok(HttpResponse::Ok().finish())
}

/// DELETE route for `/api/v1/account/{account_id}`
pub async fn delete_account(
    _pool: web::Data<Pool>,
    identity: Identity,
    _account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    println!("Delete is not supported!");

    Ok(HttpResponse::MethodNotAllowed().finish())
}

#[derive(Debug, Deserialize)]
pub struct AccountBarcode {
    pub barcode: String,
}
#[derive(Debug, Deserialize)]
pub struct AccountNfc {
    pub nfc: String,
    pub writeable: bool,
}

/// PUT route for `/api/v1/account/{account_id}/barcode`
pub async fn put_account_barcode(
    pool: web::Data<Pool>,
    identity: Identity,
    data: web::Json<AccountBarcode>,
    account_id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    let conn = &pool.get()?;
    let server_account = Account::get(&conn, &account_id)?;

    authentication_barcode::register(&conn, &server_account, &data.barcode)?;

    Ok(HttpResponse::Ok().finish())
}

/// DELETE route for `/api/v1/account/{account_id}/barcode`
pub async fn delete_account_barcode(
    pool: web::Data<Pool>,
    identity: Identity,
    account_id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    let conn = &pool.get()?;
    let server_account = Account::get(&conn, &account_id)?;

    authentication_barcode::remove(&conn, &server_account)?;

    Ok(HttpResponse::Ok().finish())
}

/// PUT route for `/api/v1/account/{account_id}/nfc`
pub async fn put_account_nfc(
    pool: web::Data<Pool>,
    identity: Identity,
    data: web::Json<AccountNfc>,
    account_id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    let conn = &pool.get()?;
    let server_account = Account::get(&conn, &account_id)?;

    authentication_nfc::register(&conn, &server_account, &data.nfc, data.writeable)?;

    Ok(HttpResponse::Ok().finish())
}

/// DELETE route for `/api/v1/account/{account_id}/nfc`
pub async fn delete_account_nfc(
    pool: web::Data<Pool>,
    identity: Identity,
    account_id: web::Path<Uuid>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    let conn = &pool.get()?;
    let server_account = Account::get(&conn, &account_id)?;

    authentication_nfc::remove(&conn, &server_account)?;

    Ok(HttpResponse::Ok().finish())
}
