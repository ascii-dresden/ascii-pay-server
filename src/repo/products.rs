use std::fs::File;
use std::io::{Read, Write};

use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{Category, Permission, Price, Product};
use crate::utils::{
    fuzzy_vec_match, uuid_to_str, DatabaseConnection, Money, ServiceError, ServiceResult,
};
use log::warn;
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
        let image = entity
            .image
            .as_ref()
            .map(|_| format!("/api/v1/product/{}/image", uuid_to_str(entity.id)));
        Self {
            id: entity.id,
            name: entity.name,
            category: entity.category.map(CategoryOutput::from),
            image,
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
    database_conn: &DatabaseConnection,
    identity: &Identity,
    search: Option<&str>,
) -> ServiceResult<Vec<SearchElement<ProductOutput>>> {
    identity.require_account_or_cert(Permission::Member)?;

    let search = match search {
        Some(s) => s.to_owned(),
        None => "".to_owned(),
    };

    let lower_search = search.trim().to_ascii_lowercase();
    let entities: Vec<SearchElement<ProductOutput>> = Product::all(database_conn)?
        .into_iter()
        .filter_map(|p| serach_product(p, &lower_search))
        .collect();

    Ok(entities)
}

pub fn get_product(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
) -> ServiceResult<ProductOutput> {
    identity.require_account_or_cert(Permission::Member)?;

    let entity = Product::get(database_conn, id)?;
    Ok(entity.into())
}

pub fn get_product_image(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
) -> ServiceResult<String> {
    identity.require_account_or_cert(Permission::Member)?;

    let entity = Product::get(database_conn, id)?;
    entity.get_image()
}

pub fn create_product(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    input: ProductInput,
) -> ServiceResult<ProductOutput> {
    identity.require_account_or_cert(Permission::Member)?;

    let category = if let Some(category) = input.category {
        Some(Category::get(database_conn, category)?)
    } else {
        None
    };

    let mut entity = Product::create(database_conn, &input.name, category)?;

    entity.barcode = input.barcode.clone();
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

pub fn update_product(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
    input: ProductInput,
) -> ServiceResult<ProductOutput> {
    identity.require_account_or_cert(Permission::Member)?;

    let mut entity = Product::get(database_conn, id)?;

    let category = if let Some(category) = input.category {
        Some(Category::get(database_conn, category)?)
    } else {
        None
    };

    entity.name = input.name;
    entity.barcode = input.barcode;
    entity.category = category;

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

pub fn remove_product_image(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
) -> ServiceResult<()> {
    identity.require_account_or_cert(Permission::Member)?;

    let mut entity = Product::get(database_conn, id)?;
    entity.remove_image(database_conn)?;

    Ok(())
}

pub fn set_product_image(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
    _filename: &str,
    content_type: Option<&str>,
    content: &mut File,
) -> ServiceResult<()> {
    identity.require_account_or_cert(Permission::Member)?;

    let mut entity = Product::get(database_conn, id)?;

    let mut file_extension = "png";

    if let Some(content_type) = content_type {
        match content_type.to_ascii_lowercase().as_str() {
            "image/png" => file_extension = "png",
            "image/jpg" => file_extension = "jpg",
            "image/jpeg" => file_extension = "jpg",
            _ => {}
        }
    }

    let mut file = entity.set_image(database_conn, file_extension)?;

    let chunk_size = 0x4000;
    loop {
        let mut chunk = Vec::with_capacity(chunk_size);
        let n = std::io::Read::by_ref(content)
            .take(chunk_size as u64)
            .read_to_end(&mut chunk)?;
        if n == 0 {
            break;
        }

        let mut pos = 0;
        while pos < chunk.len() {
            let bytes_written = file.write(&chunk[pos..])?;
            pos += bytes_written;
        }

        if n < chunk_size {
            break;
        }
    }

    file.flush()?;
    Ok(())
}

/// DELETE route for `/api/v1/product/{product_id}`
pub fn delete_product(
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
