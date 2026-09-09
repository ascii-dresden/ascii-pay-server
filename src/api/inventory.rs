use aide::axum::routing::{get_with, put_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::{Path, Query};
use axum::Json;
use chrono::Utc;
use reqwest::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::models;
use crate::request_state::RequestState;

use super::products::ProductDto;
use super::purchases::PurchaseDto;
use super::shopping_list::ShoppingListItemDto;

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route("/inventory", get_with(get_inventory, get_inventory_docs))
        .api_route(
            "/inventory/checks",
            get_with(list_inventory_checks, list_inventory_checks_docs)
                .post_with(create_inventory_check, create_inventory_check_docs),
        )
        .api_route(
            "/inventory/check/:id",
            get_with(get_inventory_check, get_inventory_check_docs)
                .delete_with(delete_inventory_check, delete_inventory_check_docs),
        )
        .api_route(
            "/inventory/check/:id/item/:product_id",
            put_with(set_inventory_check_item, set_inventory_check_item_docs),
        )
        .api_route(
            "/inventory/check/:id/complete",
            aide::axum::routing::post_with(complete_inventory_check, complete_inventory_check_docs),
        )
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct InventoryLastPurchaseDto {
    pub purchase_id: u64,
    pub timestamp: String,
    pub container_size: i32,
    pub container_cents: i32,
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct InventoryRowDto {
    pub product: ProductDto,
    pub target_quantity: i32,
    pub last_counted_quantity: Option<i32>,
    pub last_counted_at: Option<String>,
    /// Last count plus units of finalized purchases minus sales since the count. `null` when
    /// the product was never counted.
    pub estimated_quantity: Option<i64>,
    pub last_purchase: Option<InventoryLastPurchaseDto>,
}

impl From<&models::InventoryRow> for InventoryRowDto {
    fn from(value: &models::InventoryRow) -> Self {
        Self {
            product: (&value.product).into(),
            target_quantity: value.target_quantity,
            last_counted_quantity: value.last_counted_quantity,
            last_counted_at: value.last_counted_at.map(|ts| format!("{:?}", ts)),
            estimated_quantity: value.estimated_quantity,
            last_purchase: value
                .last_purchase
                .as_ref()
                .map(|lp| InventoryLastPurchaseDto {
                    purchase_id: lp.purchase_id,
                    timestamp: format!("{:?}", lp.timestamp),
                    container_size: lp.container_size,
                    container_cents: lp.container_cents,
                }),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InventoryCheckStateDto {
    Draft,
    Completed,
}

impl From<models::InventoryCheckState> for InventoryCheckStateDto {
    fn from(value: models::InventoryCheckState) -> Self {
        match value {
            models::InventoryCheckState::Draft => InventoryCheckStateDto::Draft,
            models::InventoryCheckState::Completed => InventoryCheckStateDto::Completed,
        }
    }
}

impl From<InventoryCheckStateDto> for models::InventoryCheckState {
    fn from(value: InventoryCheckStateDto) -> Self {
        match value {
            InventoryCheckStateDto::Draft => models::InventoryCheckState::Draft,
            InventoryCheckStateDto::Completed => models::InventoryCheckState::Completed,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct InventoryCheckSummaryDto {
    pub id: u64,
    pub state: InventoryCheckStateDto,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub started_by_account_id: Option<u64>,
    pub completed_by_account_id: Option<u64>,
    pub generated_purchase_id: Option<u64>,
    pub note: String,
    pub item_count: u64,
    pub counted_count: u64,
}

impl From<&models::InventoryCheck> for InventoryCheckSummaryDto {
    fn from(value: &models::InventoryCheck) -> Self {
        Self {
            id: value.id,
            state: value.state.into(),
            started_at: format!("{:?}", value.started_at),
            completed_at: value.completed_at.map(|ts| format!("{:?}", ts)),
            started_by_account_id: value.started_by_account_id,
            completed_by_account_id: value.completed_by_account_id,
            generated_purchase_id: value.generated_purchase_id,
            note: value.note.to_owned(),
            item_count: value.item_count,
            counted_count: value.counted_count,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct InventoryCheckItemDto {
    pub product: ProductDto,
    pub target_quantity: i32,
    pub counted_quantity: Option<i32>,
}

impl From<&models::InventoryCheckItem> for InventoryCheckItemDto {
    fn from(value: &models::InventoryCheckItem) -> Self {
        Self {
            product: (&value.product).into(),
            target_quantity: value.target_quantity,
            counted_quantity: value.counted_quantity,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct InventoryCheckDto {
    #[serde(flatten)]
    pub summary: InventoryCheckSummaryDto,
    /// Ordered by product category, then name.
    pub items: Vec<InventoryCheckItemDto>,
}

impl From<&models::InventoryCheck> for InventoryCheckDto {
    fn from(value: &models::InventoryCheck) -> Self {
        Self {
            summary: value.into(),
            items: value.items.iter().map(Into::into).collect(),
        }
    }
}

pub async fn get_inventory(mut state: RequestState) -> ServiceResult<Json<Vec<InventoryRowDto>>> {
    let rows = state.db.get_inventory().await?;
    Ok(Json(rows.iter().map(Into::into).collect()))
}

fn get_inventory_docs(op: TransformOperation) -> TransformOperation {
    op.description("Inventory overview: one row per stocked product, ordered by category and name.")
        .tag("inventory")
        .response::<200, Json<Vec<InventoryRowDto>>>()
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct InventoryCheckListQuery {
    /// Only return checks in this state.
    pub state: Option<InventoryCheckStateDto>,
}

pub async fn list_inventory_checks(
    mut state: RequestState,
    Query(query): Query<InventoryCheckListQuery>,
) -> ServiceResult<Json<Vec<InventoryCheckSummaryDto>>> {
    let checks = state
        .db
        .get_inventory_checks(query.state.map(Into::into))
        .await?;
    Ok(Json(checks.iter().map(Into::into).collect()))
}

fn list_inventory_checks_docs(op: TransformOperation) -> TransformOperation {
    op.description("List inventory checks, newest first. Optionally filtered by state.")
        .tag("inventory")
        .response::<200, Json<Vec<InventoryCheckSummaryDto>>>()
}

#[derive(Debug, Default, PartialEq, Deserialize, JsonSchema)]
pub struct CreateInventoryCheckDto {
    #[serde(default)]
    pub note: Option<String>,
}

async fn create_inventory_check(
    mut state: RequestState,
    form: Json<CreateInventoryCheckDto>,
) -> ServiceResult<Json<InventoryCheckDto>> {
    let account = state.session_require_purchaser_or_admin()?;

    let note = form.0.note.unwrap_or_default();
    let check = state
        .db
        .create_inventory_check(&note, Some(account.id))
        .await?;
    Ok(Json(InventoryCheckDto::from(&check)))
}

fn create_inventory_check_docs(op: TransformOperation) -> TransformOperation {
    op.description("Start a new inventory check with one item per stocked product.")
        .tag("inventory")
        .response::<200, Json<InventoryCheckDto>>()
        .response_with::<409, (), _>(|res| res.description("A draft check already exists!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

pub async fn get_inventory_check(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<InventoryCheckDto>> {
    let check = state.db.get_inventory_check_by_id(id).await?;

    if let Some(check) = check {
        return Ok(Json(InventoryCheckDto::from(&check)));
    }

    Err(ServiceError::NotFound)
}

fn get_inventory_check_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get an inventory check by id, including its items.")
        .tag("inventory")
        .response::<200, Json<InventoryCheckDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested check does not exist!"))
}

#[derive(Debug, Default, PartialEq, Deserialize, JsonSchema)]
pub struct SetInventoryCheckItemDto {
    /// Counted units, or `null` to clear the count.
    #[serde(default)]
    pub counted_quantity: Option<i32>,
}

async fn set_inventory_check_item(
    mut state: RequestState,
    Path((id, product_id)): Path<(u64, u64)>,
    form: Json<SetInventoryCheckItemDto>,
) -> ServiceResult<Json<InventoryCheckItemDto>> {
    state.session_require_purchaser_or_admin()?;

    let counted_quantity = form.0.counted_quantity;
    if counted_quantity.is_some_and(|q| q < 0) {
        return Err(ServiceError::BadRequest(
            "counted_quantity must not be negative".to_string(),
        ));
    }

    let item = state
        .db
        .set_inventory_check_item_count(id, product_id, counted_quantity)
        .await?;
    Ok(Json(InventoryCheckItemDto::from(&item)))
}

fn set_inventory_check_item_docs(op: TransformOperation) -> TransformOperation {
    op.description("Record the counted quantity of a product in a draft check.")
        .tag("inventory")
        .response::<200, Json<InventoryCheckItemDto>>()
        .response_with::<400, (), _>(|res| res.description("Negative quantity!"))
        .response_with::<404, (), _>(|res| {
            res.description("The requested check or product does not exist!")
        })
        .response_with::<409, (), _>(|res| res.description("The check is completed!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn delete_inventory_check(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<StatusCode> {
    state.session_require_purchaser_or_admin()?;

    let check = state
        .db
        .get_inventory_check_by_id(id)
        .await?
        .ok_or(ServiceError::NotFound)?;

    if matches!(check.state, models::InventoryCheckState::Completed) {
        state.session_require_admin()?;
    }

    state.db.delete_inventory_check(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_inventory_check_docs(op: TransformOperation) -> TransformOperation {
    op.description("Delete an inventory check. Completed checks can only be deleted by admins.")
        .tag("inventory")
        .response_with::<204, (), _>(|res| res.description("The check was successfully deleted!"))
        .response_with::<404, (), _>(|res| res.description("The requested check does not exist!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct CompleteInventoryCheckDto {
    /// Put every counted product below its target on the shared shopping list. Defaults to
    /// true: counting the stock and going shopping are separate moments, and the shopping list
    /// is what carries the result from one to the other.
    #[serde(default = "default_true")]
    pub add_to_shopping_list: bool,
    /// Note prefix of the generated shopping list entries; `<counted>/<target>` is appended.
    /// Defaults to `Inventory <YYYY-MM-DD>`.
    #[serde(default)]
    pub note: Option<String>,
    /// Additionally create a draft purchase for those products. Off by default; only useful
    /// when the shopping trip starts right after the count.
    #[serde(default)]
    pub generate_purchase: bool,
    /// Store of the generated purchase (defaults to an empty string).
    #[serde(default)]
    pub store: Option<String>,
    /// Name of the generated purchase (defaults to `Inventory <YYYY-MM-DD>`).
    #[serde(default)]
    pub name: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for CompleteInventoryCheckDto {
    fn default() -> Self {
        Self {
            add_to_shopping_list: true,
            note: None,
            generate_purchase: false,
            store: None,
            name: None,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct CompleteInventoryCheckResultDto {
    pub check: InventoryCheckDto,
    /// The generated draft purchase, `null` unless one was explicitly requested.
    pub purchase: Option<PurchaseDto>,
    /// The shopping list entries that were created. Products that already have an open entry
    /// are skipped, so this can be shorter than the list of articles below target.
    pub shopping_list_items: Vec<ShoppingListItemDto>,
}

async fn complete_inventory_check(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<CompleteInventoryCheckDto>,
) -> ServiceResult<Json<CompleteInventoryCheckResultDto>> {
    let account = state.session_require_purchaser_or_admin()?;

    let form = form.0;
    let default_label = format!("Inventory {}", Utc::now().format("%Y-%m-%d"));
    let completion = models::InventoryCheckCompletion {
        completed_by_account_id: Some(account.id),
        add_to_shopping_list: form.add_to_shopping_list,
        shopping_list_note: form.note.unwrap_or_else(|| default_label.clone()),
        generate_purchase: form.generate_purchase,
        purchase_store: form.store.unwrap_or_default(),
        purchase_name: form.name.unwrap_or(default_label),
    };

    let result = state.db.complete_inventory_check(id, completion).await?;
    Ok(Json(CompleteInventoryCheckResultDto {
        check: InventoryCheckDto::from(&result.check),
        purchase: result.purchase.as_ref().map(PurchaseDto::from),
        shopping_list_items: result
            .shopping_list_items
            .iter()
            .map(ShoppingListItemDto::from)
            .collect(),
    }))
}

fn complete_inventory_check_docs(op: TransformOperation) -> TransformOperation {
    op.description("Complete a draft check. Every counted product below its target is put on the shared shopping list (products that already have an open entry are skipped); a draft purchase is only created when explicitly requested. Uncounted items are skipped.")
        .tag("inventory")
        .response::<200, Json<CompleteInventoryCheckResultDto>>()
        .response_with::<404, (), _>(|res| res.description("The requested check does not exist!"))
        .response_with::<409, (), _>(|res| res.description("The check is already completed!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}
