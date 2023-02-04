use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::error::{ServiceError, ServiceResult};
use crate::models;

use super::accounts::CoinAmountDto;

pub fn router() -> Router<Database> {
    Router::new()
        .route(
            "/product/:id",
            get(get_product).put(update_product).delete(delete_product),
        )
        .route("/products", get(list_products).post(create_product))
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ProductDto {
    pub id: u64,
    pub name: String,
    pub price: CoinAmountDto,
    pub bonus: CoinAmountDto,
    pub nickname: Option<String>,
    pub barcode: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
}

impl From<&models::Product> for ProductDto {
    fn from(value: &models::Product) -> Self {
        Self {
            id: value.id.to_owned(),
            name: value.name.to_owned(),
            price: (&value.price).into(),
            bonus: (&value.bonus).into(),
            nickname: value.nickname.to_owned(),
            barcode: value.barcode.to_owned(),
            category: value.category.to_owned(),
            tags: value.tags.to_owned(),
        }
    }
}

pub async fn list_products(
    State(database): State<Database>,
) -> ServiceResult<Json<Vec<ProductDto>>> {
    let products = database.get_all_products().await?;
    let mut product_list = Vec::<ProductDto>::new();

    for product in products.iter() {
        product_list.push(product.into());
    }

    Ok(Json(product_list))
}

pub async fn get_product(
    State(database): State<Database>,
    Path(id): Path<u64>,
) -> ServiceResult<Json<ProductDto>> {
    let product = database.get_product_by_id(id).await?;

    if let Some(product) = product {
        return Ok(Json(ProductDto::from(&product)));
    }

    Err(ServiceError::NotFound)
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct SaveProductDto {
    pub name: String,
    pub price: CoinAmountDto,
    pub bonus: CoinAmountDto,
    pub nickname: Option<String>,
    pub barcode: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
}

async fn create_product(
    State(database): State<Database>,
    form: Json<SaveProductDto>,
) -> ServiceResult<Json<ProductDto>> {
    let form = form.0;

    let product = models::Product {
        id: 0,
        name: form.name,
        price: form.price.into(),
        bonus: form.bonus.into(),
        nickname: form.nickname,
        barcode: form.barcode,
        category: form.category,
        tags: form.tags,
        image: None,
    };

    let product = database.store_product(product).await?;
    Ok(Json(ProductDto::from(&product)))
}

async fn update_product(
    State(database): State<Database>,
    Path(id): Path<u64>,
    form: Json<SaveProductDto>,
) -> ServiceResult<Json<ProductDto>> {
    let form = form.0;
    let product = database.get_product_by_id(id).await?;

    if let Some(mut product) = product {
        product.name = form.name;
        product.price = form.price.into();
        product.bonus = form.bonus.into();
        product.nickname = form.nickname;
        product.barcode = form.barcode;
        product.category = form.category;
        product.tags = form.tags;

        let product = database.store_product(product).await?;
        return Ok(Json(ProductDto::from(&product)));
    }

    Err(ServiceError::NotFound)
}

async fn delete_product(
    State(database): State<Database>,
    Path(id): Path<u64>,
) -> ServiceResult<()> {
    database.delete_product(id).await
}
