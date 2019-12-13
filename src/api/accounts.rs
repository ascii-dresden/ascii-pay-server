use crate::api::utils::Search;
use crate::core::{Pool, Account, ServiceResult, fuzzy_vec_match, Permission};
use actix_web::{web, HttpResponse};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct SearchAccount {
    #[serde(flatten)]
    pub account: Account,
    pub name_search: String,
    pub mail_search: String,
    pub permission_search: String,
}

impl SearchAccount {
    pub fn wrap(account: Account, search: &str) -> Option<SearchAccount> {
        let mut values = vec![];

        values.push(account.name.clone()
            .unwrap_or_else(|| "".to_owned())
        );

        values.push(account.mail.clone()
            .unwrap_or_else(|| "".to_owned())
        );

        values.push(match account.permission {
            Permission::DEFAULT => "",
            Permission::MEMBER => "member",
            Permission::ADMIN => "admin",
        }.to_owned());

        let mut result = if search.is_empty() {
            values
        } else {
            match fuzzy_vec_match(search, &values) {
                Some(r) => r,
                None => return None
            }
        };

        Some(SearchAccount{
            account,
            permission_search: result.pop().expect(""),
            mail_search: result.pop().expect(""),
            name_search: result.pop().expect(""),
        })
    }
}

/// GET route for `/accounts`
pub async fn get_accounts(
    pool: web::Data<Pool>,
    query: web::Query<Search>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let search = match &query.search {
        Some(s) => s.clone(),
        None => "".to_owned()
    };

    let lower_search = search.trim().to_ascii_lowercase();
    let search_accounts: Vec<SearchAccount> = Account::all(&conn)?
        .into_iter()
        .filter_map(|p| SearchAccount::wrap(p, &lower_search))
        .collect();

    Ok(HttpResponse::Ok().json(&search_accounts))
}

/// GET route for `/account/{account_id}`
pub async fn get_account_edit(
    pool: web::Data<Pool>,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

    Ok(HttpResponse::Ok().json(&account))
}
