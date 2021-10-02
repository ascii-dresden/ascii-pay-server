use crate::identity_service::{Identity, IdentityRequire};
use crate::model::session::{Session, get_onetime_session};
use crate::model::{Account, Permission};
use crate::utils::{DatabaseConnection, Money, RedisConnection, ServiceError, ServiceResult, fuzzy_vec_match};
use log::warn;
use uuid::Uuid;

use super::SearchElement;

#[derive(Debug, Deserialize, InputObject)]
pub struct AccountInput {
    pub minimum_credit: Money,
    pub name: String,
    pub mail: Option<String>,
    pub username: Option<String>,
    pub account_number: Option<String>,
    pub permission: Permission,
    pub receives_monthly_report: bool,
    pub allow_nfc_registration: bool,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct AccountOutput {
    pub id: Uuid,
    pub credit: Money,
    pub minimum_credit: Money,
    pub name: String,
    pub mail: Option<String>,
    pub username: Option<String>,
    pub account_number: Option<String>,
    pub permission: Permission,
    pub receives_monthly_report: bool,
    pub allow_nfc_registration: bool,
}

impl From<Account> for AccountOutput {
    fn from(entity: Account) -> Self {
        Self {
            id: entity.id,
            credit: entity.credit,
            minimum_credit: entity.minimum_credit,
            name: entity.name,
            mail: entity.mail,
            username: entity.username,
            account_number: entity.account_number,
            permission: entity.permission,
            receives_monthly_report: entity.receives_monthly_report,
            allow_nfc_registration: entity.allow_nfc_registration,
        }
    }
}

fn search_account(entity: Account, search: &str) -> Option<SearchElement<AccountOutput>> {
    let values = vec![
        entity
            .id
            .to_hyphenated()
            .encode_upper(&mut Uuid::encode_buffer())
            .to_owned(),
        entity.name.clone(),
        entity.mail.clone().unwrap_or_else(|| "".to_owned()),
        entity.username.clone().unwrap_or_else(|| "".to_owned()),
        entity
            .account_number
            .clone()
            .unwrap_or_else(|| "".to_owned()),
        match entity.permission {
            Permission::Default => "",
            Permission::Member => "member",
            Permission::Admin => "admin",
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

    let mut search_element = SearchElement::new(entity.into());

    search_element.add_highlight("permission", result.pop().expect(""));
    search_element.add_highlight("account_number", result.pop().expect(""));
    search_element.add_highlight("username", result.pop().expect(""));
    search_element.add_highlight("mail", result.pop().expect(""));
    search_element.add_highlight("name", result.pop().expect(""));
    search_element.add_highlight("id", result.pop().expect(""));

    Some(search_element)
}

pub fn get_accounts(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    search: Option<&str>,
) -> ServiceResult<Vec<SearchElement<AccountOutput>>> {
    identity.require_account_or_cert(Permission::Member)?;

    let search = match search {
        Some(s) => s.to_owned(),
        None => "".to_owned(),
    };

    let lower_search = search.trim().to_ascii_lowercase();
    let entities: Vec<SearchElement<AccountOutput>> = Account::all(database_conn)?
        .into_iter()
        .filter_map(|a| search_account(a, &lower_search))
        .collect();

    Ok(entities)
}

pub fn get_account(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
) -> ServiceResult<AccountOutput> {
    identity.require_account_or_cert(Permission::Member)?;

    let entity = Account::get(database_conn, id)?;
    Ok(entity.into())
}

pub fn get_account_by_access_token(
    database_conn: &DatabaseConnection,
    redis_conn: &mut RedisConnection,
    identity: &Identity,
    account_access_token: Session,
) -> ServiceResult<AccountOutput> {
    identity.require_cert()?;

    let entity = get_onetime_session(database_conn, redis_conn, &account_access_token)?;
    Ok(entity.into())
}

pub fn create_account(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    input: AccountInput,
) -> ServiceResult<AccountOutput> {
    identity.require_account_or_cert(Permission::Member)?;

    let mut entity = Account::create(database_conn, &input.name, input.permission)?;

    entity.minimum_credit = input.minimum_credit;
    entity.mail = input.mail.clone();
    entity.username = input.username.clone();
    entity.account_number = input.account_number.clone();
    entity.receives_monthly_report = input.receives_monthly_report;
    entity.allow_nfc_registration = input.allow_nfc_registration;

    entity.update(database_conn)?;

    Ok(entity.into())
}

pub fn update_account(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
    input: AccountInput,
) -> ServiceResult<AccountOutput> {
    identity.require_account_or_cert(Permission::Member)?;

    let mut entity = Account::get(database_conn, id)?;

    entity.minimum_credit = input.minimum_credit;
    entity.name = input.name.clone();
    entity.mail = input.mail.clone();
    entity.username = input.username.clone();
    entity.account_number = input.account_number.clone();
    entity.permission = input.permission;
    entity.receives_monthly_report = input.receives_monthly_report;
    entity.allow_nfc_registration = input.allow_nfc_registration;

    entity.update(database_conn)?;

    Ok(entity.into())
}

pub fn delete_account(
    _database_conn: &DatabaseConnection,
    identity: &Identity,
    _id: Uuid,
) -> ServiceResult<()> {
    identity.require_account_or_cert(Permission::Member)?;

    warn!("Delete is not supported!");

    Err(ServiceError::InternalServerError(
        "Method not supported",
        "Delete operation is not supported!".to_owned(),
    ))
}
