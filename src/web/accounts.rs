use crate::core::{
    authentication_password, fuzzy_vec_match, Account, Money, Permission, Pool, ServiceError,
    ServiceResult,
};
use crate::identity_policy::{Action, RetrievedAccount};
use crate::login_required;
use crate::web::utils::{EmptyToNone, HbData, Search};
use actix_web::{http, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;
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

#[derive(Debug, Serialize)]
pub struct SearchAccount {
    #[serde(flatten)]
    pub account: Account,
    pub id_search: String,
    pub name_search: String,
    pub mail_search: String,
    pub permission_search: String,
}

impl SearchAccount {
    pub fn wrap(account: Account, search: &str) -> Option<SearchAccount> {
        let values = vec![
            account
                .id
                .to_hyphenated()
                .encode_upper(&mut Uuid::encode_buffer())
                .to_owned(),
            account.name.clone().unwrap_or_else(|| "".to_owned()),
            account.mail.clone().unwrap_or_else(|| "".to_owned()),
            match account.permission {
                Permission::DEFAULT => "",
                Permission::MEMBER => "member",
                Permission::ADMIN => "admin",
            }
            .to_owned(),
        ];

        let mut result = if search.is_empty() {
            values
        } else {
            match fuzzy_vec_match(search, &values) {
                Some(r) => r,
                None => return None,
            }
        };

        Some(SearchAccount {
            account,
            permission_search: result.pop().expect(""),
            mail_search: result.pop().expect(""),
            name_search: result.pop().expect(""),
            id_search: result.pop().expect(""),
        })
    }
}

/// GET route for `/accounts`
pub async fn get_accounts(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars>,
    logged_account: RetrievedAccount,
    query: web::Query<Search>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

    let conn = &pool.get()?;

    let search = match &query.search {
        Some(s) => s.clone(),
        None => "".to_owned(),
    };

    let lower_search = search.trim().to_ascii_lowercase();
    let search_accounts: Vec<SearchAccount> = Account::all(&conn)?
        .into_iter()
        .filter_map(|a| SearchAccount::wrap(a, &lower_search))
        .collect();

    let body = HbData::new(&request)
        .with_account(logged_account)
        .with_data("search", &search)
        .with_data("accounts", &search_accounts)
        .render(&hb, "account_list")?;

    Ok(HttpResponse::Ok().body(body))
}

/// GET route for `/account/{account_id}`
pub async fn get_account_edit(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars>,
    logged_account: RetrievedAccount,
    account_id: web::Path<String>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

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
    logged_account: RetrievedAccount,
    account: web::Form<FormAccount>,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

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
    logged_account: RetrievedAccount,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let logged_account = login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

    let body = HbData::new(&request)
        .with_account(logged_account)
        .render(&hb, "account_create")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/account/create`
pub async fn post_account_create(
    pool: web::Data<Pool>,
    logged_account: RetrievedAccount,
    account: web::Form<FormAccount>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

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
    logged_account: RetrievedAccount,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

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
    logged_account: RetrievedAccount,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

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
    logged_account: RetrievedAccount,
    _account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    let _logged_account = login_required!(logged_account, Permission::MEMBER, Action::REDIRECT);

    println!("Delete is not supported!");

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/accounts")
        .finish())
}
