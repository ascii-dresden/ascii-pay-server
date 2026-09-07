use aide::axum::routing::{get_with, post_with, put_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, NaiveDate};
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
            "/purchase/:id/finalize",
            post_with(finalize_purchase, finalize_purchase_docs),
        )
        .api_route(
            "/purchase/:id/reopen",
            post_with(reopen_purchase, reopen_purchase_docs),
        )
        .api_route(
            "/purchase/:id/items",
            post_with(add_purchase_item, add_purchase_item_docs),
        )
        .api_route(
            "/purchase/:id/item/:item_id",
            put_with(update_purchase_item, update_purchase_item_docs)
                .delete_with(delete_purchase_item, delete_purchase_item_docs),
        )
        .api_route(
            "/purchases",
            get_with(list_purchases, list_purchases_docs)
                .post_with(create_purchase, create_purchase_docs),
        )
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PurchaseStateDto {
    Draft,
    Finalized,
}

impl From<models::PurchaseState> for PurchaseStateDto {
    fn from(value: models::PurchaseState) -> Self {
        match value {
            models::PurchaseState::Draft => PurchaseStateDto::Draft,
            models::PurchaseState::Finalized => PurchaseStateDto::Finalized,
        }
    }
}

impl From<PurchaseStateDto> for models::PurchaseState {
    fn from(value: PurchaseStateDto) -> Self {
        match value {
            PurchaseStateDto::Draft => models::PurchaseState::Draft,
            PurchaseStateDto::Finalized => models::PurchaseState::Finalized,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct PurchaseItemDto {
    pub id: u64,
    pub name: String,
    pub container_size: i32,
    pub container_count: i32,
    pub container_cents: i32,
    /// Best-before date as `YYYY-MM-DD`.
    pub best_before: Option<String>,
    pub barcode: Option<String>,
    pub collected: bool,
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
            best_before: value.best_before.map(|d| d.to_string()),
            barcode: value.barcode.to_owned(),
            collected: value.collected,
            product,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct PurchaseDto {
    pub id: u64,
    pub name: String,
    pub store: String,
    pub timestamp: String,
    pub state: PurchaseStateDto,
    pub purchased_by_account_id: Option<u64>,
    pub finalized_at: Option<String>,
    pub finalized_by_account_id: Option<u64>,
    pub items: Vec<PurchaseItemDto>,
}

impl From<&models::Purchase> for PurchaseDto {
    fn from(value: &models::Purchase) -> Self {
        Self {
            id: value.id.to_owned(),
            name: value.name.to_owned(),
            store: value.store.to_owned(),
            timestamp: format!("{:?}", value.timestamp),
            state: value.state.into(),
            purchased_by_account_id: value.purchased_by_account_id.to_owned(),
            finalized_at: value.finalized_at.map(|ts| format!("{:?}", ts)),
            finalized_by_account_id: value.finalized_by_account_id.to_owned(),
            items: value.items.iter().map(|i| i.into()).collect(),
        }
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct PurchaseListQuery {
    /// Only return purchases in this state.
    pub state: Option<PurchaseStateDto>,
}

pub async fn list_purchases(
    mut state: RequestState,
    Query(query): Query<PurchaseListQuery>,
) -> ServiceResult<Json<Vec<PurchaseDto>>> {
    let purchases = state.db.get_purchases(query.state.map(Into::into)).await?;
    Ok(Json(purchases.iter().map(|t| t.into()).collect()))
}

fn list_purchases_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all purchases, newest first. Optionally filtered by state.")
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
    /// Optional label of the purchase (defaults to an empty string).
    #[serde(default)]
    pub name: Option<String>,
    pub store: String,
    pub timestamp: String,
    /// Initial state, only honoured on creation (defaults to `draft`).
    #[serde(default)]
    pub state: Option<PurchaseStateDto>,
    #[serde(default)]
    pub purchased_by_account_id: Option<u64>,
    /// When present on update, the whole item list is replaced.
    #[serde(default)]
    pub items: Option<Vec<SavePurchaseItemDto>>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SavePurchaseItemDto {
    pub name: String,
    pub container_size: i32,
    pub container_count: i32,
    pub container_cents: i32,
    /// Best-before date as `YYYY-MM-DD`.
    #[serde(default)]
    pub best_before: Option<String>,
    #[serde(default)]
    pub barcode: Option<String>,
    #[serde(default = "default_true")]
    pub collected: bool,
    #[serde(default)]
    pub product_id: Option<u64>,
}

async fn create_purchase(
    mut state: RequestState,
    form: Json<SavePurchaseDto>,
) -> ServiceResult<Json<PurchaseDto>> {
    let account = state.session_require_purchaser_or_admin()?;

    let form = form.0;

    let timestamp = DateTime::parse_from_rfc3339(&form.timestamp)?;
    let purchase_state: models::PurchaseState =
        form.state.unwrap_or(PurchaseStateDto::Draft).into();
    let items = resolve_items(&mut state, form.items.as_deref().unwrap_or(&[])).await?;

    let finalized_by_account_id = match purchase_state {
        models::PurchaseState::Draft => None,
        models::PurchaseState::Finalized => Some(account.id),
    };

    let purchase = models::Purchase {
        id: 0,
        name: form.name.unwrap_or_default(),
        purchased_by_account_id: form.purchased_by_account_id,
        store: form.store,
        timestamp: timestamp.into(),
        state: purchase_state,
        finalized_at: None,
        finalized_by_account_id,
        items,
    };

    let purchase = state.db.create_purchase(purchase).await?;
    Ok(Json(PurchaseDto::from(&purchase)))
}

fn create_purchase_docs(op: TransformOperation) -> TransformOperation {
    op.description("Create a new purchase. The `state` field is honoured, `items` is optional.")
        .tag("purchases")
        .response::<200, Json<PurchaseDto>>()
        .response_with::<400, (), _>(|res| res.description("Malformed timestamp or date!"))
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
    let items = match form.items.as_deref() {
        Some(items) => Some(resolve_items(&mut state, items).await?),
        None => None,
    };

    if let Some(mut purchase) = purchase {
        if let Some(name) = form.name {
            purchase.name = name;
        }
        purchase.purchased_by_account_id = form.purchased_by_account_id;
        purchase.store = form.store;
        purchase.timestamp = timestamp.into();

        let replace_items = items.is_some();
        if let Some(items) = items {
            purchase.items = items;
        }

        let purchase = state.db.update_purchase(purchase, replace_items).await?;
        return Ok(Json(PurchaseDto::from(&purchase)));
    }

    Err(ServiceError::NotFound)
}

fn update_purchase_docs(op: TransformOperation) -> TransformOperation {
    op.description("Update the header fields of a draft purchase. When `items` is present the whole item list is replaced. The `state` field is ignored.")
        .tag("purchases")
        .response::<200, Json<PurchaseDto>>()
        .response_with::<400, (), _>(|res| res.description("Malformed timestamp or date!"))
        .response_with::<404, (), _>(|res| {
            res.description("The requested purchase does not exist!")
        })
        .response_with::<409, (), _>(|res| res.description("The purchase is finalized!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn delete_purchase(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<StatusCode> {
    state.session_require_purchaser_or_admin()?;

    let purchase = state
        .db
        .get_purchase_by_id(id)
        .await?
        .ok_or(ServiceError::NotFound)?;

    if matches!(purchase.state, models::PurchaseState::Finalized) {
        state.session_require_admin()?;
    }

    state.db.delete_purchase(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_purchase_docs(op: TransformOperation) -> TransformOperation {
    op.description(
        "Delete an existing purchase. Finalized purchases can only be deleted by admins.",
    )
    .tag("purchases")
    .response_with::<204, (), _>(|res| res.description("The purchase was successfully deleted!"))
    .response_with::<404, (), _>(|res| res.description("The requested purchase does not exist!"))
    .response_with::<401, (), _>(|res| res.description("Missing login!"))
    .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
    .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn finalize_purchase(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<PurchaseDto>> {
    let account = state.session_require_purchaser_or_admin()?;

    let purchase = state.db.finalize_purchase(id, Some(account.id)).await?;
    Ok(Json(PurchaseDto::from(&purchase)))
}

fn finalize_purchase_docs(op: TransformOperation) -> TransformOperation {
    op.description("Finalize a draft purchase. Items that were not collected are removed.")
        .tag("purchases")
        .response::<200, Json<PurchaseDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested purchase does not exist!")
        })
        .response_with::<409, (), _>(|res| res.description("The purchase is already finalized!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn reopen_purchase(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<PurchaseDto>> {
    state.session_require_admin()?;

    let purchase = state.db.reopen_purchase(id).await?;
    Ok(Json(PurchaseDto::from(&purchase)))
}

fn reopen_purchase_docs(op: TransformOperation) -> TransformOperation {
    op.description("Reopen a finalized purchase as a draft.")
        .tag("purchases")
        .response::<200, Json<PurchaseDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested purchase does not exist!")
        })
        .response_with::<409, (), _>(|res| res.description("The purchase is not finalized!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}

async fn add_purchase_item(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SavePurchaseItemDto>,
) -> ServiceResult<Json<PurchaseItemDto>> {
    state.session_require_purchaser_or_admin()?;

    let item = resolve_item(&mut state, &form.0).await?;
    let item = state.db.add_purchase_item(id, item).await?;
    Ok(Json(PurchaseItemDto::from(&item)))
}

fn add_purchase_item_docs(op: TransformOperation) -> TransformOperation {
    op.description("Add an item to a draft purchase.")
        .tag("purchases")
        .response::<200, Json<PurchaseItemDto>>()
        .response_with::<400, (), _>(|res| res.description("Malformed best-before date!"))
        .response_with::<404, (), _>(|res| {
            res.description("The requested purchase does not exist!")
        })
        .response_with::<409, (), _>(|res| res.description("The purchase is finalized!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn update_purchase_item(
    mut state: RequestState,
    Path((id, item_id)): Path<(u64, u64)>,
    form: Json<SavePurchaseItemDto>,
) -> ServiceResult<Json<PurchaseItemDto>> {
    state.session_require_purchaser_or_admin()?;

    let item = resolve_item(&mut state, &form.0).await?;
    let item = state.db.update_purchase_item(id, item_id, item).await?;
    Ok(Json(PurchaseItemDto::from(&item)))
}

fn update_purchase_item_docs(op: TransformOperation) -> TransformOperation {
    op.description("Update an item of a draft purchase.")
        .tag("purchases")
        .response::<200, Json<PurchaseItemDto>>()
        .response_with::<400, (), _>(|res| res.description("Malformed best-before date!"))
        .response_with::<404, (), _>(|res| {
            res.description("The requested purchase or item does not exist!")
        })
        .response_with::<409, (), _>(|res| res.description("The purchase is finalized!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn delete_purchase_item(
    mut state: RequestState,
    Path((id, item_id)): Path<(u64, u64)>,
) -> ServiceResult<StatusCode> {
    state.session_require_purchaser_or_admin()?;

    state.db.delete_purchase_item(id, item_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_purchase_item_docs(op: TransformOperation) -> TransformOperation {
    op.description("Delete an item of a draft purchase.")
        .tag("purchases")
        .response_with::<204, (), _>(|res| res.description("The item was successfully deleted!"))
        .response_with::<404, (), _>(|res| {
            res.description("The requested purchase or item does not exist!")
        })
        .response_with::<409, (), _>(|res| res.description("The purchase is finalized!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

async fn resolve_item(
    state: &mut RequestState,
    item_dto: &SavePurchaseItemDto,
) -> ServiceResult<models::PurchaseItem> {
    let best_before = match item_dto.best_before.as_deref() {
        Some(date) => Some(NaiveDate::parse_from_str(date, "%Y-%m-%d")?),
        None => None,
    };

    let mut item = models::PurchaseItem {
        id: 0,
        name: item_dto.name.clone(),
        container_size: item_dto.container_size,
        container_count: item_dto.container_count,
        container_cents: item_dto.container_cents,
        best_before,
        barcode: item_dto.barcode.clone(),
        collected: item_dto.collected,
        product: None,
    };

    if let Some(product_id) = item_dto.product_id {
        item.product = state.db.get_product_by_id(product_id).await?;
    }

    Ok(item)
}

async fn resolve_items(
    state: &mut RequestState,
    items_dto: &[SavePurchaseItemDto],
) -> ServiceResult<Vec<models::PurchaseItem>> {
    let mut items: Vec<models::PurchaseItem> = Vec::with_capacity(items_dto.len());

    for item_dto in items_dto {
        items.push(resolve_item(state, item_dto).await?);
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{Duration, Utc};
    use sqlx::PgPool;

    use super::*;
    use crate::database::{AppState, DatabaseConnection};
    use crate::models::{Account, AuthMethodType, CoinAmount, Role, Session};

    async fn request_state(app_state: &AppState, account: Option<Account>) -> RequestState {
        let session = account.map(|account| Session {
            account,
            token: "test".to_string(),
            auth_method: AuthMethodType::PasswordBased,
            valid_until: Utc::now() + Duration::minutes(30),
            is_single_use: false,
        });
        RequestState {
            db: DatabaseConnection {
                connection: app_state.pool.acquire().await.unwrap(),
            },
            session,
            challenge_storage: app_state.ascii_mifare_challenge.clone(),
        }
    }

    async fn store_account(db: &mut DatabaseConnection, name: &str, role: Role) -> Account {
        db.store_account(Account {
            id: 0,
            balance: CoinAmount(HashMap::new()),
            name: name.to_string(),
            email: format!("{name}@example.org"),
            role,
            auth_methods: vec![],
            enable_monthly_mail_report: false,
            enable_automatic_stamp_usage: false,
            status: None,
        })
        .await
        .unwrap()
    }

    fn draft_purchase() -> models::Purchase {
        models::Purchase {
            id: 0,
            name: String::new(),
            store: "Store".to_string(),
            timestamp: Utc::now(),
            state: models::PurchaseState::Draft,
            purchased_by_account_id: None,
            finalized_at: None,
            finalized_by_account_id: None,
            items: vec![],
        }
    }

    #[sqlx::test]
    async fn delete_finalized_purchase_requires_admin(pool: PgPool) {
        let app_state = AppState::from_pool(pool).await;
        let mut db = DatabaseConnection {
            connection: app_state.pool.acquire().await.unwrap(),
        };

        let purchaser = store_account(&mut db, "purchaser", Role::Purchaser).await;
        let admin = store_account(&mut db, "admin", Role::Admin).await;
        let member = store_account(&mut db, "member", Role::Member).await;

        let draft = db.create_purchase(draft_purchase()).await.unwrap();
        let finalized = db.create_purchase(draft_purchase()).await.unwrap();
        db.finalize_purchase(finalized.id, Some(admin.id))
            .await
            .unwrap();
        let finalized2 = db.create_purchase(draft_purchase()).await.unwrap();
        db.finalize_purchase(finalized2.id, Some(admin.id))
            .await
            .unwrap();

        // no session / wrong role
        let state = request_state(&app_state, None).await;
        assert_eq!(
            delete_purchase(state, Path(draft.id)).await,
            Err(ServiceError::Unauthorized("Missing login!"))
        );
        let state = request_state(&app_state, Some(member.clone())).await;
        assert_eq!(
            delete_purchase(state, Path(draft.id)).await,
            Err(ServiceError::Forbidden)
        );

        // purchaser: drafts yes, finalized no
        let state = request_state(&app_state, Some(purchaser.clone())).await;
        assert_eq!(
            delete_purchase(state, Path(finalized.id)).await,
            Err(ServiceError::Forbidden)
        );
        let state = request_state(&app_state, Some(purchaser.clone())).await;
        assert_eq!(
            delete_purchase(state, Path(draft.id)).await,
            Ok(StatusCode::NO_CONTENT)
        );
        assert!(db.get_purchase_by_id(finalized.id).await.unwrap().is_some());
        assert!(db.get_purchase_by_id(draft.id).await.unwrap().is_none());

        // admin: everything
        let state = request_state(&app_state, Some(admin.clone())).await;
        assert_eq!(
            delete_purchase(state, Path(finalized.id)).await,
            Ok(StatusCode::NO_CONTENT)
        );
        let state = request_state(&app_state, Some(admin.clone())).await;
        assert_eq!(
            delete_purchase(state, Path(finalized.id)).await,
            Err(ServiceError::NotFound)
        );

        // reopen is admin only
        let state = request_state(&app_state, Some(purchaser.clone())).await;
        assert_eq!(
            reopen_purchase(state, Path(finalized2.id)).await.err(),
            Some(ServiceError::Forbidden)
        );
        let state = request_state(&app_state, Some(admin.clone())).await;
        let reopened = reopen_purchase(state, Path(finalized2.id)).await.unwrap();
        assert_eq!(reopened.0.state, PurchaseStateDto::Draft);
        assert_eq!(reopened.0.finalized_at, None);
    }

    #[sqlx::test]
    async fn item_dates_are_validated_and_echoed(pool: PgPool) {
        let app_state = AppState::from_pool(pool).await;
        let mut db = DatabaseConnection {
            connection: app_state.pool.acquire().await.unwrap(),
        };
        let admin = store_account(&mut db, "admin", Role::Admin).await;
        let draft = db.create_purchase(draft_purchase()).await.unwrap();

        let item = |best_before: Option<&str>| SavePurchaseItemDto {
            name: "Thing".to_string(),
            container_size: 1,
            container_count: 1,
            container_cents: 1,
            best_before: best_before.map(str::to_string),
            barcode: None,
            collected: true,
            product_id: None,
        };

        let state = request_state(&app_state, Some(admin.clone())).await;
        let result = add_purchase_item(state, Path(draft.id), Json(item(Some("31.01.2027"))))
            .await
            .err();
        assert!(matches!(result, Some(ServiceError::BadRequest(_))));

        let state = request_state(&app_state, Some(admin.clone())).await;
        let created = add_purchase_item(state, Path(draft.id), Json(item(Some("2027-01-31"))))
            .await
            .unwrap();
        assert_eq!(created.0.best_before.as_deref(), Some("2027-01-31"));
        assert!(created.0.collected);

        // after finalizing, adding is a conflict
        db.finalize_purchase(draft.id, Some(admin.id))
            .await
            .unwrap();
        let state = request_state(&app_state, Some(admin.clone())).await;
        let result = add_purchase_item(state, Path(draft.id), Json(item(None)))
            .await
            .err();
        assert!(matches!(result, Some(ServiceError::Conflict(_))));
    }
}
