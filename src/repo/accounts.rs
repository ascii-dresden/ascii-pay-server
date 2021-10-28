use crate::identity_service::{Identity, IdentityRequire};
use crate::model::session::{get_onetime_session, Session};
use crate::model::{Account, Permission};
use crate::utils::{
    fuzzy_vec_match, DatabaseConnection, Money, RedisConnection, ServiceError, ServiceResult,
};
use log::warn;
use uuid::Uuid;

use super::SearchElement;

#[derive(Debug, Deserialize, InputObject)]
pub struct AccountCreateInput {
    pub name: String,
    pub permission: Permission,
    pub username: Option<String>,
    pub mail: Option<String>,
    pub account_number: Option<String>,
    pub minimum_credit: Option<Money>,
    pub use_digital_stamps: Option<bool>,
    pub receives_monthly_report: Option<bool>,
}

#[derive(Debug, Deserialize, InputObject)]
pub struct AccountUpdateInput {
    pub minimum_credit: Option<Money>,
    pub name: Option<String>,
    pub mail: Option<String>,
    pub username: Option<String>,
    pub account_number: Option<String>,
    pub permission: Option<Permission>,
    pub use_digital_stamps: Option<bool>,
    pub receives_monthly_report: Option<bool>,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct AccountOutput {
    pub id: Uuid,
    pub credit: Money,
    pub minimum_credit: Money,
    pub name: String,
    pub mail: String,
    pub username: String,
    pub account_number: String,
    pub permission: Permission,
    pub use_digital_stamps: bool,
    pub coffee_stamps: i32,
    pub bottle_stamps: i32,
    pub receives_monthly_report: bool,
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
            use_digital_stamps: entity.use_digital_stamps,
            coffee_stamps: entity.coffee_stamps,
            bottle_stamps: entity.bottle_stamps,
            receives_monthly_report: entity.receives_monthly_report,
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
        entity.mail.clone(),
        entity.username.clone(),
        entity.account_number.clone(),
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
    input: AccountCreateInput,
) -> ServiceResult<AccountOutput> {
    if let Permission::Admin = input.permission {
        identity.require_account(Permission::Admin)?;
    } else {
        identity.require_account_or_cert(Permission::Member)?;
    }

    let mut entity = Account::create(database_conn, &input.name, input.permission)?;

    if let Some(value) = input.minimum_credit {
        entity.minimum_credit = value;
    }
    if let Some(value) = input.mail {
        entity.mail = value;
    }
    if let Some(value) = input.username {
        entity.username = value;
    }
    if let Some(value) = input.account_number {
        entity.account_number = value;
    }
    if let Some(value) = input.use_digital_stamps {
        entity.use_digital_stamps = value;
    }
    if let Some(value) = input.receives_monthly_report {
        entity.receives_monthly_report = value;
    }

    entity.update(database_conn)?;

    Ok(entity.into())
}

pub fn update_account(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
    input: AccountUpdateInput,
) -> ServiceResult<AccountOutput> {
    identity.require_account_or_cert(Permission::Member)?;

    let mut entity = Account::get(database_conn, id)?;

    if let Permission::Admin = entity.permission {
        identity.require_account(Permission::Admin)?;
    }

    if let Some(value) = input.minimum_credit {
        entity.minimum_credit = value;
    }
    if let Some(value) = input.name {
        entity.name = value;
    }
    if let Some(value) = input.mail {
        entity.mail = value;
    }
    if let Some(value) = input.username {
        entity.username = value;
    }
    if let Some(value) = input.account_number {
        entity.account_number = value;
    }
    if let Some(value) = input.permission {
        if let Permission::Admin = value {
            identity.require_account(Permission::Admin)?;
        }

        entity.permission = value;
    }
    if let Some(value) = input.use_digital_stamps {
        entity.use_digital_stamps = value;
    }
    if let Some(value) = input.receives_monthly_report {
        entity.receives_monthly_report = value;
    }

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
