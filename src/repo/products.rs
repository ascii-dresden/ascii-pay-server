use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{
    fuzzy_vec_match, Category, DbConnection, Money, Permission, Price, Product, ServiceError,
    ServiceResult,
};
use uuid::Uuid;

use super::categories::CategoryOutput;
use super::prices::{PriceInput, PriceOutput};
use super::SearchElement;

#[derive(Debug, Deserialize, InputObject)]
pub struct ProductInput {
    pub name: String,
    pub category: Option<Uuid>,
    pub prices: Vec<PriceInput>,
    pub barcode: Option<String>,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct ProductOutput {
    pub id: Uuid,
    pub name: String,
    pub category: Option<CategoryOutput>,
    pub image: Option<String>,
    pub prices: Vec<PriceOutput>,
    pub current_price: Option<Money>,
    pub barcode: Option<String>,
}

impl From<Product> for ProductOutput {
    fn from(entity: Product) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            category: entity.category.map(CategoryOutput::from),
            image: entity.image,
            prices: entity.prices.into_iter().map(PriceOutput::from).collect(),
            current_price: entity.current_price,
            barcode: entity.barcode,
        }
    }
}

fn serach_product(entity: Product, search: &str) -> Option<SearchElement<ProductOutput>> {
    let values = vec![
        entity.name.clone(),
        entity
            .category
            .clone()
            .map(|v| v.name)
            .unwrap_or_else(|| "".to_owned()),
        entity
            .current_price
            .map(|v| format!("{:.2}€", (v as f32) / 100.0))
            .unwrap_or_else(|| "".to_owned()),
        entity.barcode.clone().unwrap_or_else(String::new),
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

    search_element.add_highlight("barcode", result.pop().expect(""));
    search_element.add_highlight("current_price", result.pop().expect(""));
    search_element.add_highlight("category", result.pop().expect(""));
    search_element.add_highlight("name", result.pop().expect(""));

    Some(search_element)
}

pub fn get_products(
    conn: &DbConnection,
    identity: &Identity,
    search: Option<&str>,
) -> ServiceResult<Vec<SearchElement<ProductOutput>>> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    let search = match search {
        Some(s) => s.to_owned(),
        None => "".to_owned(),
    };

    let lower_search = search.trim().to_ascii_lowercase();
    let entities: Vec<SearchElement<ProductOutput>> = Product::all(conn)?
        .into_iter()
        .filter_map(|p| serach_product(p, &lower_search))
        .collect();

    Ok(entities)
}

pub fn get_product(
    conn: &DbConnection,
    identity: &Identity,
    id: Uuid,
) -> ServiceResult<ProductOutput> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    let entity = Product::get(conn, &id)?;
    Ok(entity.into())
}

pub fn create_product(
    conn: &DbConnection,
    identity: &Identity,
    input: ProductInput,
) -> ServiceResult<ProductOutput> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    let category = if let Some(category) = &input.category {
        Some(Category::get(conn, &category)?)
    } else {
        None
    };

    let mut entity = Product::create(conn, &input.name, category)?;

    entity.barcode = input.barcode.clone();
    entity.update(conn)?;

    entity.update_prices(
        conn,
        &input
            .prices
            .into_iter()
            .map(Price::from)
            .collect::<Vec<_>>(),
    )?;

    Ok(entity.into())
}

pub fn update_product(
    conn: &DbConnection,
    identity: &Identity,
    id: Uuid,
    input: ProductInput,
) -> ServiceResult<ProductOutput> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    let mut entity = Product::get(conn, &id)?;

    let category = if let Some(category) = &input.category {
        Some(Category::get(conn, category)?)
    } else {
        None
    };

    entity.name = input.name;
    entity.barcode = input.barcode;
    entity.category = category;

    entity.update(conn)?;

    entity.update_prices(
        conn,
        &input
            .prices
            .into_iter()
            .map(Price::from)
            .collect::<Vec<_>>(),
    )?;

    Ok(entity.into())
}

/// DELETE route for `/api/v1/product/{product_id}`
pub fn delete_product(_conn: &DbConnection, identity: &Identity, _id: Uuid) -> ServiceResult<()> {
    identity.require_account_or_cert(Permission::MEMBER)?;

    println!("Delete is not supported!");

    Err(ServiceError::InternalServerError(
        "Method not supported",
        "Delete operation is not supported!".to_owned(),
    ))
}
