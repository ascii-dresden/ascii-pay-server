use crate::core::{Account, Pool, ServiceResult};
use crate::web::admin::accounts::SearchAccount;
use crate::web::utils::Search;
use actix_web::{web, HttpResponse};
use uuid::Uuid;

/// GET route for `/api/v1/accounts`
pub async fn get_accounts(
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
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

/// GET route for `/api/v1/account/{account_id}`
pub async fn get_account_edit(
    pool: web::Data<Pool>,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

    Ok(HttpResponse::Ok().json(&account))
}
