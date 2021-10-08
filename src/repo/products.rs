use std::fs::File;
use std::io::{Read, Write};

use crate::identity_service::{Identity, IdentityRequire};
use crate::model::{Category, Permission, Product, StampType};
use crate::utils::{
    fuzzy_vec_match, uuid_to_str, DatabaseConnection, Money, ServiceError, ServiceResult,
};
use log::warn;
use uuid::Uuid;

use super::categories::CategoryOutput;
use super::SearchElement;

#[derive(Debug, Deserialize, InputObject)]
pub struct ProductInput {
    pub name: String,
    pub price: Option<Money>,
    pub pay_with_stamps: Option<StampType>,
    pub give_stamps: Option<StampType>,
    pub category_id: Uuid,
    pub barcode: Option<String>,
    pub ordering: Option<i32>,
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct ProductOutput {
    pub id: Uuid,
    pub name: String,
    pub price: Option<Money>,
    pub pay_with_stamps: Option<StampType>,
    pub give_stamps: Option<StampType>,
    pub category: CategoryOutput,
    pub image: Option<String>,
    pub barcode: Option<String>,
    pub ordering: Option<i32>,
}

impl From<(Product, Category)> for ProductOutput {
    fn from(entity: (Product, Category)) -> Self {
        let image = entity
            .0
            .image
            .as_ref()
            .map(|_| format!("/api/v1/product/{}/image", uuid_to_str(entity.0.id)));
        Self {
            id: entity.0.id,
            name: entity.0.name,
            price: entity.0.price,
            pay_with_stamps: entity.0.pay_with_stamps,
            give_stamps: entity.0.give_stamps,
            category: entity.1.into(),
            image,
            barcode: entity.0.barcode,
            ordering: entity.0.ordering,
        }
    }
}

fn serach_product(
    entity: (Product, Category),
    search: &str,
) -> Option<SearchElement<ProductOutput>> {
    let values = vec![
        entity.0.name.clone(),
        entity.1.name.clone(),
        entity
            .0
            .price
            .map(|v| format!("{:.2}€", (v as f32) / 100.0))
            .unwrap_or_else(|| "".to_owned()),
        entity.0.barcode.clone().unwrap_or_else(String::new),
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
    search_element.add_highlight("price", result.pop().expect(""));
    search_element.add_highlight("category", result.pop().expect(""));
    search_element.add_highlight("name", result.pop().expect(""));

    Some(search_element)
}

pub fn get_products(
    database_conn: &DatabaseConnection,
    _identity: &Identity,
    search: Option<&str>,
) -> ServiceResult<Vec<SearchElement<ProductOutput>>> {
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
    _identity: &Identity,
    id: Uuid,
) -> ServiceResult<ProductOutput> {
    let entity = Product::get(database_conn, id)?;
    Ok(entity.into())
}

pub fn get_product_image(
    database_conn: &DatabaseConnection,
    _identity: &Identity,
    id: Uuid,
) -> ServiceResult<String> {
    let entity = Product::get(database_conn, id)?;
    entity.0.get_image()
}

pub fn create_product(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    input: ProductInput,
) -> ServiceResult<ProductOutput> {
    identity.require_account(Permission::Member)?;

    let category = Category::get(database_conn, input.category_id)?;
    let mut entity = Product::create(database_conn, &input.name, &category)?;

    entity.price = input.price;
    entity.pay_with_stamps = input.pay_with_stamps;
    entity.give_stamps = input.give_stamps;
    entity.ordering = input.ordering;
    entity.barcode = input.barcode;

    entity.update(database_conn)?;

    Ok((entity, category).into())
}

pub fn update_product(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
    input: ProductInput,
) -> ServiceResult<ProductOutput> {
    identity.require_account(Permission::Member)?;

    let category = Category::get(database_conn, input.category_id)?;
    let (mut entity, _) = Product::get(database_conn, id)?;

    entity.name = input.name;
    entity.category_id = category.id;
    entity.price = input.price;
    entity.pay_with_stamps = input.pay_with_stamps;
    entity.give_stamps = input.give_stamps;
    entity.ordering = input.ordering;
    entity.barcode = input.barcode;

    entity.update(database_conn)?;

    Ok((entity, category).into())
}

pub fn remove_product_image(
    database_conn: &DatabaseConnection,
    identity: &Identity,
    id: Uuid,
) -> ServiceResult<()> {
    identity.require_account(Permission::Member)?;

    let mut entity = Product::get(database_conn, id)?;
    entity.0.remove_image(database_conn)?;

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
    identity.require_account(Permission::Member)?;

    let (mut entity, _) = Product::get(database_conn, id)?;

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
    identity.require_account(Permission::Member)?;

    warn!("Delete is not supported!");

    Err(ServiceError::InternalServerError(
        "Method not supported",
        "Delete operation is not supported!".to_owned(),
    ))
}
