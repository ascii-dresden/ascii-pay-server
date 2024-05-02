use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::Path;
use axum::Json;
use chrono::DateTime;
use reqwest::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::models;
use crate::request_state::RequestState;

use super::products::ProductDto;

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/product/:id/purchases",
            get_with(list_purchases_by_product, list_purchases_by_product_docs),
        )
        .api_route(
            "/purchase/:id",
            get_with(get_purchase, get_purchase_docs)
                .put_with(update_purchase, update_purchase_docs)
                .delete_with(delete_purchase, delete_purchase_docs),
        )
        .api_route(
            "/purchases",
            get_with(list_purchases, list_purchases_docs)
                .post_with(create_purchase, create_purchase_docs),
        )
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct PurchaseItemDto {
    pub id: u64,
    pub name: String,
    pub container_size: i32,
    pub container_count: i32,
    pub container_cents: i32,
    pub product: Option<ProductDto>,
}

impl From<&models::PurchaseItem> for PurchaseItemDto {
    fn from(value: &models::PurchaseItem) -> Self {
        let product = value.product.as_ref().map(|product| product.into());

        Self {
            id: value.id,
            name: value.name.to_owned(),
            container_size: value.container_size,
            container_count: value.container_count,
            container_cents: value.container_cents,
            product,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct PurchaseDto {
    pub id: u64,
    pub store: String,
    pub timestamp: String,
    pub purchased_by_account_id: Option<u64>,
    pub items: Vec<PurchaseItemDto>,
}

impl From<&models::Purchase> for PurchaseDto {
    fn from(value: &models::Purchase) -> Self {
        Self {
            id: value.id.to_owned(),
            store: value.store.to_owned(),
            timestamp: format!("{:?}", value.timestamp),
            purchased_by_account_id: value.purchased_by_account_id.to_owned(),
            items: value.items.iter().map(|i| i.into()).collect(),
        }
    }
}

pub async fn list_purchases(mut state: RequestState) -> ServiceResult<Json<Vec<PurchaseDto>>> {
    let purchases = state.db.get_purchases().await?;
    Ok(Json(purchases.iter().map(|t| t.into()).collect()))
}

fn list_purchases_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all purchases.")
        .tag("purchases")
        .response::<200, Json<Vec<PurchaseDto>>>()
}

pub async fn list_purchases_by_product(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<Vec<PurchaseDto>>> {
    let purchases = state.db.get_purchases_by_product_id(id).await?;

    if let Some(purchases) = purchases {
        return Ok(Json(purchases.iter().map(|t| t.into()).collect()));
    }

    Err(ServiceError::NotFound)
}

fn list_purchases_by_product_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all purchases that contain the given product.")
        .tag("purchases")
        .response::<200, Json<Vec<PurchaseDto>>>()
        .response_with::<404, (), _>(|res| res.description("The requested product does not exist!"))
}

pub async fn get_purchase(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<PurchaseDto>> {
    let purchase = state.db.get_purchase_by_id(id).await?;

    if let Some(purchase) = purchase {
        return Ok(Json(PurchaseDto::from(&purchase)));
    }

    Err(ServiceError::NotFound)
}

fn get_purchase_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get purchase by id.")
        .tag("purchases")
        .response::<200, Json<PurchaseDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested purchase does not exist!")
        })
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SavePurchaseDto {
    pub id: u64,
    pub store: String,
    pub timestamp: String,
    pub purchased_by_account_id: Option<u64>,
    pub items: Vec<SavePurchaseItemDto>,
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SavePurchaseItemDto {
    pub id: u64,
    pub name: String,
    pub container_size: i32,
    pub container_count: i32,
    pub container_cents: i32,
    pub product_id: Option<u64>,
}

async fn create_purchase(
    mut state: RequestState,
    form: Json<SavePurchaseDto>,
) -> ServiceResult<Json<PurchaseDto>> {
    state.session_require_purchaser_or_admin()?;

    let form = form.0;

    let timestamp = DateTime::parse_from_rfc3339(&form.timestamp)?;
    let items = resolve_items(&mut state, &form.items).await?;

    let purchase = models::Purchase {
        id: 0,
        purchased_by_account_id: form.purchased_by_account_id,
        store: form.store,
        timestamp: timestamp.into(),
        items,
    };

    let purchase = state.db.store_purchase(purchase).await?;
    Ok(Json(PurchaseDto::from(&purchase)))
}

fn create_purchase_docs(op: TransformOperation) -> TransformOperation {
    op.description("Create a new purchase.")
        .tag("purchases")
        .response::<200, Json<PurchaseDto>>()
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn update_purchase(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SavePurchaseDto>,
) -> ServiceResult<Json<PurchaseDto>> {
    state.session_require_purchaser_or_admin()?;

    let form = form.0;
    let purchase = state.db.get_purchase_by_id(id).await?;

    let timestamp = DateTime::parse_from_rfc3339(&form.timestamp)?;
    let items = resolve_items(&mut state, &form.items).await?;

    if let Some(mut purchase) = purchase {
        purchase.purchased_by_account_id = form.purchased_by_account_id;
        purchase.store = form.store;
        purchase.timestamp = timestamp.into();
        purchase.items = items;

        let purchase = state.db.store_purchase(purchase).await?;
        return Ok(Json(PurchaseDto::from(&purchase)));
    }

    Err(ServiceError::NotFound)
}

fn update_purchase_docs(op: TransformOperation) -> TransformOperation {
    op.description("Update an existing purchase.")
        .tag("purchases")
        .response::<200, Json<PurchaseDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested purchase does not exist!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn delete_purchase(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<StatusCode> {
    state.session_require_purchaser_or_admin()?;

    state.db.delete_purchase(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_purchase_docs(op: TransformOperation) -> TransformOperation {
    op.description("Delete an existing purchase.")
        .tag("purchases")
        .response_with::<204, (), _>(|res| {
            res.description("The purchase was successfully deleted!")
        })
        .response_with::<404, (), _>(|res| {
            res.description("The requested purchase does not exist!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn resolve_items(
    state: &mut RequestState,
    items_dto: &[SavePurchaseItemDto],
) -> ServiceResult<Vec<models::PurchaseItem>> {
    let mut items: Vec<models::PurchaseItem> = Vec::new();

    for item_dto in items_dto {
        let mut item = models::PurchaseItem {
            id: 0,
            name: item_dto.name.clone(),
            container_size: item_dto.container_size,
            container_count: item_dto.container_count,
            container_cents: item_dto.container_cents,
            product: None,
        };

        if let Some(product_id) = item_dto.product_id {
            item.product = state.db.get_product_by_id(product_id).await?;
        }

        items.push(item);
    }

    Ok(items)
}
