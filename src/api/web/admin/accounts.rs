use crate::api::web::utils::{EmptyToNone, HbData, IsJson, Search};
use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{
    authentication_barcode, authentication_nfc, authentication_password, fuzzy_vec_match, Account,
    Money, Permission, Pool, ServiceError, ServiceResult,
};
use actix_web::{http, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;
use uuid::Uuid;

use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct FormAccount {
    pub id: String,
    pub name: String,
    pub mail: String,
    pub username: String,
    pub account_number: String,
    pub minimum_credit: f32,
    pub permission: Permission,
    pub receives_monthly_report: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DisplayType {
    TEXT,
    EDIT,
    LINK,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationMethod {
    pub name: String,
    pub display: Option<(DisplayType, String)>,
    pub action: Option<(String, String)>,
    pub id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchAccount {
    #[serde(flatten)]
    pub account: Account,
    pub id_search: String,
    pub name_search: String,
    pub mail_search: String,
    pub username_search: String,
    pub account_number_search: String,
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
            account.name.clone(),
            account.mail.clone().unwrap_or_else(|| "".to_owned()),
            account.username.clone().unwrap_or_else(|| "".to_owned()),
            account
                .account_number
                .clone()
                .unwrap_or_else(|| "".to_owned()),
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
            account_number_search: result.pop().expect(""),
            username_search: result.pop().expect(""),
            mail_search: result.pop().expect(""),
            name_search: result.pop().expect(""),
            id_search: result.pop().expect(""),
        })
    }
}

/// GET route for `/admin/accounts`
pub async fn get_accounts(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    query: web::Query<Search>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let account = if request.is_json() {
        identity.require_account(Permission::MEMBER)?
    } else {
        identity.require_account_with_redirect(Permission::MEMBER)?
    };

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

    if request.is_json() {
        Ok(HttpResponse::Ok().json(search_accounts))
    } else {
        let body = HbData::new(&request)
            .with_account(account)
            .with_data("search", &search)
            .with_data("accounts", &search_accounts)
            .render(&hb, "admin_account_list")?;

        Ok(HttpResponse::Ok().body(body))
    }
}

/// GET route for `/admin/account/{account_id}`
pub async fn get_account_edit(
    pool: web::Data<Pool>,
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    account_id: web::Path<String>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let identity_account = if request.is_json() {
        identity.require_account(Permission::MEMBER)?
    } else {
        identity.require_account_with_redirect(Permission::MEMBER)?
    };

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;
    let has_mail_address = account.mail.is_some();

    let mut authentication_methods: Vec<AuthenticationMethod> = vec![];

    if let Some(invitation_link) = authentication_password::get_invitation_link(&conn, &account)? {
        authentication_methods.push(AuthenticationMethod {
            name: "Invite link".to_owned(),
            display: Some((DisplayType::LINK, format!("/register/{}", invitation_link))),
            action: Some((
                "Revoke".to_owned(),
                format!("/admin/account/revoke/{}", &account.id),
            )),
            id: None,
        });
    }
    if authentication_password::has_password(&conn, &account)? {
        authentication_methods.push(AuthenticationMethod {
            name: "Password".to_owned(),
            display: Some((DisplayType::TEXT, "Password exists".to_owned())),
            action: Some((
                "Revoke".to_owned(),
                format!("/admin/account/revoke/{}", &account.id),
            )),
            id: None,
        });
    }
    if authentication_methods.is_empty() {
        authentication_methods.push(AuthenticationMethod {
            name: "Password".to_owned(),
            display: None,
            action: Some((
                "Create invitation".to_owned(),
                format!("/admin/account/invite/{}", &account.id),
            )),
            id: None,
        });
    }

    for (barcode_id, barcode) in authentication_barcode::get_barcodes(&conn, &account)?
        .into_iter()
        .enumerate()
    {
        authentication_methods.push(AuthenticationMethod {
            name: "Barcode".to_owned(),
            display: Some((DisplayType::TEXT, barcode)),
            action: Some((
                "Delete".to_owned(),
                format!("/admin/account/remove-barcode/{}", &account.id),
            )),
            id: Some(format!("barcode-{}", barcode_id)),
        });
    }
    if authentication_methods.len() == 1 {
        authentication_methods.push(AuthenticationMethod {
            name: "Add Barcode".to_owned(),
            display: Some((DisplayType::EDIT, "".to_owned())),
            action: None,
            id: Some("barcode-new".to_owned()),
        });
    }

    for (nfc_id, nfc) in authentication_nfc::get_nfcs(&conn, &account)?
        .into_iter()
        .enumerate()
    {
        let card_id = nfc.card_id.clone();

        let name = if nfc.is_secure() {
            "NFC (secure)"
        } else if nfc.need_write_key(&conn)? {
            "NFC (pending)"
        } else {
            "NFC (insecure)"
        }
        .to_owned();

        authentication_methods.push(AuthenticationMethod {
            name,
            display: Some((DisplayType::TEXT, card_id)),
            action: Some((
                "Delete".to_owned(),
                format!("/admin/account/remove-nfc/{}", &account.id),
            )),
            id: Some(format!("nfc-{}", nfc_id)),
        });
    }
    if authentication_methods.len() == 2 {
        authentication_methods.push(AuthenticationMethod {
            name: "Add NFC".to_owned(),
            display: Some((DisplayType::EDIT, "".to_owned())),
            action: None,
            id: Some("nfc-new".to_owned()),
        });
    }

    let body = HbData::new(&request)
        .with_account(identity_account)
        .with_data("account", &account)
        .with_data("authentication_methods", &authentication_methods)
        .with_data("has_mail_address", &has_mail_address)
        .render(&hb, "admin_account_edit")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/admin/account/{account_id}`
pub async fn post_account_edit(
    pool: web::Data<Pool>,
    identity: Identity,
    account: web::Form<FormAccount>,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    if *account_id != account.id {
        return Err(ServiceError::BadRequest(
            "Id missmage",
            "The product id of the url and the form do not match!".to_owned(),
        ));
    }

    let conn = &pool.get()?;

    let mut server_account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;

    server_account.name = account.name.clone();
    let new_mail = account.mail.empty_to_none();

    // only enable monthly reports when mail address is existent
    server_account.receives_monthly_report =
        new_mail.is_some() && (account.receives_monthly_report == Some("on".to_string()));
    server_account.mail = new_mail;
    server_account.username = account.username.empty_to_none();
    server_account.account_number = account.account_number.empty_to_none();
    server_account.permission = account.permission;
    server_account.minimum_credit = (account.minimum_credit * 100.0) as Money;

    server_account.update(&conn)?;

    let mut reauth = false;

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
            reauth = true;
        }
    }

    let location = if reauth {
        "/admin/accounts?reauthenticate"
    } else {
        "/admin/accounts"
    };

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, location)
        .finish())
}

/// GET route for `/admin/account/create`
pub async fn get_account_create(
    hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let identity_account = identity.require_account_with_redirect(Permission::MEMBER)?;

    let body = HbData::new(&request)
        .with_account(identity_account)
        .render(&hb, "admin_account_create")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/admin/account/create`
pub async fn post_account_create(
    pool: web::Data<Pool>,
    identity: Identity,
    account: web::Form<FormAccount>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let mut server_account = Account::create(&conn, &account.name, account.permission)?;

    server_account.mail = account.mail.empty_to_none();
    server_account.username = account.username.empty_to_none();
    server_account.account_number = account.account_number.empty_to_none();
    server_account.minimum_credit = (account.minimum_credit * 100.0) as Money;

    server_account.update(&conn)?;

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/admin/account/{}", server_account.id),
        )
        .finish())
}

/// GET route for `/admin/account/invite/{account_id}`
pub async fn invite_get(
    pool: web::Data<Pool>,
    identity: Identity,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;
    authentication_password::create_invitation_link(&conn, &account)?;

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/admin/account/{}", account.id),
        )
        .finish())
}

/// GET route for `/admin/account/revoke/{account_id}`
pub async fn revoke_get(
    pool: web::Data<Pool>,
    identity: Identity,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;
    authentication_password::revoke_invitation_link(&conn, &account)?;
    authentication_password::remove(&conn, &account)?;

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/admin/account/{}", account.id),
        )
        .finish())
}

/// GET route for `/admin/account/remove-nfc/{account_id}`
pub async fn remove_nfc_get(
    pool: web::Data<Pool>,
    identity: Identity,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;
    authentication_nfc::remove(&conn, &account)?;

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/admin/account/{}", account.id),
        )
        .finish())
}

/// GET route for `/admin/account/remove-nfc/{account_id}`
pub async fn remove_barcode_get(
    pool: web::Data<Pool>,
    identity: Identity,
    account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    let conn = &pool.get()?;

    let account = Account::get(&conn, &Uuid::parse_str(&account_id)?)?;
    authentication_barcode::remove(&conn, &account)?;

    Ok(HttpResponse::Found()
        .header(
            http::header::LOCATION,
            format!("/admin/account/{}", account.id),
        )
        .finish())
}

/// GET route for `/admin/account/delete/{account_id}`
pub async fn delete_get(
    _hb: web::Data<Handlebars<'_>>,
    identity: Identity,
    _account_id: web::Path<String>,
) -> ServiceResult<HttpResponse> {
    identity.require_account_with_redirect(Permission::MEMBER)?;

    println!("Delete is not supported!");

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/admin/accounts")
        .finish())
}
