use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::Path;
use axum::Json;
use schemars::JsonSchema;
use serde::Serialize;

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::models;
use crate::request_state::RequestState;

use super::products::ProductDto;

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route("/purchase/:id", get_with(get_purchase, get_purchase_docs))
        .api_route("/purchases", get_with(list_purchases, list_purchases_docs))
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
