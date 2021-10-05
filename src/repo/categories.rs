use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{Category, Permission, Price};
use crate::utils::{fuzzy_vec_match, DatabaseConnection, Money, ServiceError, ServiceResult};
use log::warn;
use uuid::Uuid;

use super::prices::{PriceInput, PriceOutput};
use super::SearchElement;

#[derive(Debug, Deserialize, InputObject)]
pub struct CategoryInput {
    pub name: String,
    pub prices: Vec<PriceInput>,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct CategoryOutput {
    pub id: Uuid,
    pub name: String,
    pub prices: Vec<PriceOutput>,
    pub current_price: Option<Money>,
}

impl From<Category> for CategoryOutput {
    fn from(entity: Category) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            prices: entity.prices.into_iter().map(PriceOutput::from).collect(),
            current_price: entity.current_price,
        }
    }
}

fn search_category(entity: Category, search: &str) -> Option<SearchElement<CategoryOutput>> {
    let values = vec![
        entity.name.clone(),
        entity
            .current_price
            .map(|v| format!("{:.2}€", (v as f32) / 100.0))
            .unwrap_or_else(|| "".to_owned()),
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
    input: CategoryInput,
) -> ServiceResult<CategoryOutput> {
    identity.require_account(Permission::Admin)?;

    let mut entity = Category::create(database_conn, &input.name)?;
    entity.update_prices(
        database_conn,
        &input
            .prices
            .into_iter()
            .map(Price::from)
            .collect::<Vec<_>>(),
    )?;

    Ok(entity.into())
}

pub fn update_category(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
    input: CategoryInput,
) -> ServiceResult<CategoryOutput> {
    identity.require_account(Permission::Admin)?;

    let mut entity = Category::get(database_conn, id)?;
    entity.name = input.name.clone();
    entity.update(database_conn)?;

    entity.update_prices(
        database_conn,
        &input
            .prices
            .into_iter()
            .map(Price::from)
            .collect::<Vec<_>>(),
    )?;

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
