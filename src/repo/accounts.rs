use crate::identity_service::{Identity, IdentityRequire};
use crate::model::authentication_nfc::AuthenticationNfc;
use crate::model::session::{get_onetime_session, Session};
use crate::model::{Account, AccountJoined, Permission};
use crate::utils::{fuzzy_vec_match, DatabasePool, Money, RedisPool, ServiceError, ServiceResult};
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
pub struct AccountNfcTokenOutput {
    pub card_id: String,
    pub card_type: String,
    pub name: String,
}

impl From<AuthenticationNfc> for AccountNfcTokenOutput {
    fn from(entity: AuthenticationNfc) -> Self {
        Self {
            card_id: entity.card_id,
            card_type: entity.card_type,
            name: entity.name,
        }
    }
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
    pub is_password_set: bool,
    pub nfc_tokens: Vec<AccountNfcTokenOutput>,
}

impl From<AccountJoined> for AccountOutput {
    fn from(entity: AccountJoined) -> Self {
        Self {
            id: entity.0.id,
            credit: entity.0.credit,
            minimum_credit: entity.0.minimum_credit,
            name: entity.0.name,
            mail: entity.0.mail,
            username: entity.0.username,
            account_number: entity.0.account_number,
            permission: entity.0.permission,
            use_digital_stamps: entity.0.use_digital_stamps,
            coffee_stamps: entity.0.coffee_stamps,
            bottle_stamps: entity.0.bottle_stamps,
            receives_monthly_report: entity.0.receives_monthly_report,
            is_password_set: entity.1,
            nfc_tokens: entity.2.into_iter().map(|token| token.into()).collect(),
        }
    }
}

fn search_account(entity: AccountJoined, search: &str) -> Option<SearchElement<AccountOutput>> {
    let values = vec![
        entity
            .0
            .id
            .to_hyphenated()
            .encode_upper(&mut Uuid::encode_buffer())
            .to_owned(),
        entity.0.name.clone(),
        entity.0.mail.clone(),
        entity.0.username.clone(),
        entity.0.account_number.clone(),
        match entity.0.permission {
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

pub async fn get_accounts(
    database_pool: &DatabasePool,
    identity: &Identity,
    search: Option<&str>,
) -> ServiceResult<Vec<SearchElement<AccountOutput>>> {
    identity.require_account_or_cert(Permission::Member)?;

    let search = match search {
        Some(s) => s.to_owned(),
        None => "".to_owned(),
    };

    let lower_search = search.trim().to_ascii_lowercase();
    let entities: Vec<SearchElement<AccountOutput>> = Account::all_joined(database_pool)
        .await?
        .into_iter()
        .filter_map(|a| search_account(a, &lower_search))
        .collect();

    Ok(entities)
}

pub async fn get_account(
    database_pool: &DatabasePool,
    identity: &Identity,
    id: Uuid,
) -> ServiceResult<AccountOutput> {
    identity.require_account_or_cert(Permission::Member)?;

    let entity = Account::get_joined(database_pool, id).await?;
    Ok(entity.into())
}

pub async fn get_account_by_access_token(
    database_pool: &DatabasePool,
    redis_pool: &RedisPool,
    identity: &Identity,
    account_access_token: Session,
) -> ServiceResult<AccountOutput> {
    identity.require_cert()?;

    let entity = get_onetime_session(database_pool, redis_pool, &account_access_token).await?;

    let entity = entity.joined(database_pool).await?;
    Ok(entity.into())
}

pub async fn create_account(
    database_pool: &DatabasePool,
    identity: &Identity,
    input: AccountCreateInput,
) -> ServiceResult<AccountOutput> {
    if let Permission::Admin = input.permission {
        identity.require_account(Permission::Admin)?;
    } else {
        identity.require_account_or_cert(Permission::Member)?;
    }

    let mut entity = Account::create(database_pool, &input.name, input.permission).await?;

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

    entity.update(database_pool).await?;

    let entity = entity.joined(database_pool).await?;
    Ok(entity.into())
}

pub async fn update_account(
    database_pool: &DatabasePool,
    identity: &Identity,
    id: Uuid,
    input: AccountUpdateInput,
) -> ServiceResult<AccountOutput> {
    identity.require_account_or_cert(Permission::Member)?;

    let mut entity = Account::get(database_pool, id).await?;

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

    entity.update(database_pool).await?;

    let entity = entity.joined(database_pool).await?;
    Ok(entity.into())
}

pub fn delete_account(
    _database_pool: &DatabasePool,
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
