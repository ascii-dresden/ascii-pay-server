use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{Category, Permission, StampType};
use crate::utils::{fuzzy_vec_match, DatabaseConnection, Money, ServiceError, ServiceResult};
use log::warn;
use uuid::Uuid;

use super::SearchElement;

#[derive(Debug, Deserialize, InputObject)]
pub struct CategoryCreateInput {
    pub name: String,
    pub price: Money,
    pub pay_with_stamps: Option<StampType>,
    pub give_stamps: Option<StampType>,
    pub ordering: Option<i32>,
}

#[derive(Debug, Deserialize, InputObject)]
pub struct CategoryUpdateInput {
    pub name: Option<String>,
    pub price: Option<Money>,
    pub pay_with_stamps: Option<StampType>,
    pub give_stamps: Option<StampType>,
    pub ordering: Option<i32>,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct CategoryOutput {
    pub id: Uuid,
    pub name: String,
    pub price: Money,
    pub pay_with_stamps: StampType,
    pub give_stamps: StampType,
    pub ordering: Option<i32>,
}

impl From<Category> for CategoryOutput {
    fn from(entity: Category) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            price: entity.price,
            pay_with_stamps: entity.pay_with_stamps,
            give_stamps: entity.give_stamps,
            ordering: entity.ordering,
        }
    }
}

fn search_category(entity: Category, search: &str) -> Option<SearchElement<CategoryOutput>> {
    let values = vec![
        entity.name.clone(),
        format!("{:.2}€", (entity.price as f32) / 100.0),
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

    search_element.add_highlight("current_price", result.pop().expect(""));
    search_element.add_highlight("name", result.pop().expect(""));

    Some(search_element)
}

pub fn get_categories(
    database_conn: &DatabaseConnection,
    _identity: &Identity,
    search: Option<&str>,
) -> ServiceResult<Vec<SearchElement<CategoryOutput>>> {
    let search = match search {
        Some(s) => s.to_owned(),
        None => "".to_owned(),
    };

    let lower_search = search.trim().to_ascii_lowercase();
    let entities: Vec<SearchElement<CategoryOutput>> = Category::all(database_conn)?
        .into_iter()
        .filter_map(|c| search_category(c, &lower_search))
        .collect();

    Ok(entities)
}

pub fn get_category(
    database_conn: &DatabaseConnection,
    _identity: &Identity,
    id: Uuid,
) -> ServiceResult<CategoryOutput> {
    let entity = Category::get(database_conn, id)?;
    Ok(entity.into())
}

pub fn create_category(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    input: CategoryCreateInput,
) -> ServiceResult<CategoryOutput> {
    identity.require_account(Permission::Admin)?;

    let mut entity = Category::create(database_conn, &input.name, input.price)?;
    if let Some(value) = input.pay_with_stamps {
        entity.pay_with_stamps = value;
    }
    if let Some(value) = input.give_stamps {
        entity.give_stamps = value;
    }
    entity.ordering = input.ordering;
    entity.update(database_conn)?;

    Ok(entity.into())
}

pub fn update_category(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
    input: CategoryUpdateInput,
) -> ServiceResult<CategoryOutput> {
    identity.require_account(Permission::Admin)?;

    let mut entity = Category::get(database_conn, id)?;
    if let Some(value) = input.name {
        entity.name = value;
    }
    if let Some(value) = input.price {
        entity.price = value;
    }
    if let Some(value) = input.pay_with_stamps {
        entity.pay_with_stamps = value;
    }
    if let Some(value) = input.give_stamps {
        entity.give_stamps = value;
    }
    entity.ordering = input.ordering;
    entity.update(database_conn)?;

    Ok(entity.into())
}

pub fn delete_category(
    _database_conn: &DatabaseConnection,
    identity: &Identity,
    _id: Uuid,
) -> ServiceResult<()> {
    identity.require_account(Permission::Admin)?;

    warn!("Delete is not supported!");

    Err(ServiceError::InternalServerError(
        "Method not supported",
        "Delete operation is not supported!".to_owned(),
    ))
}
