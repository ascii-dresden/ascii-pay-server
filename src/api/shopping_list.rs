use aide::axum::routing::{get_with, post_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::{Path, Query};
use axum::Json;
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
            "/shopping-list",
            get_with(list_shopping_list_items, list_shopping_list_items_docs)
                .post_with(create_shopping_list_item, create_shopping_list_item_docs),
        )
        .api_route(
            "/shopping-list/:id",
            get_with(get_shopping_list_item, get_shopping_list_item_docs)
                .put_with(update_shopping_list_item, update_shopping_list_item_docs)
                .delete_with(delete_shopping_list_item, delete_shopping_list_item_docs),
        )
        .api_route(
            "/shopping-list/:id/done",
            post_with(
                mark_shopping_list_item_done,
                mark_shopping_list_item_done_docs,
            ),
        )
        .api_route(
            "/shopping-list/:id/reopen",
            post_with(reopen_shopping_list_item, reopen_shopping_list_item_docs),
        )
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ShoppingListStateDto {
    #[default]
    Open,
    Done,
    All,
}

impl From<ShoppingListStateDto> for models::ShoppingListState {
    fn from(value: ShoppingListStateDto) -> Self {
        match value {
            ShoppingListStateDto::Open => models::ShoppingListState::Open,
            ShoppingListStateDto::Done => models::ShoppingListState::Done,
            ShoppingListStateDto::All => models::ShoppingListState::All,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct ShoppingListItemDto {
    pub id: u64,
    pub name: String,
    pub quantity: i32,
    pub note: String,
    pub product: Option<ProductDto>,
    pub created_by_account_id: Option<u64>,
    pub created_by_name: Option<String>,
    pub created_at: String,
    pub done_at: Option<String>,
    pub done_by_account_id: Option<u64>,
    pub purchase_id: Option<u64>,
}

impl From<&models::ShoppingListItem> for ShoppingListItemDto {
    fn from(value: &models::ShoppingListItem) -> Self {
        Self {
            id: value.id,
            name: value.name.to_owned(),
            quantity: value.quantity,
            note: value.note.to_owned(),
            product: value.product.as_ref().map(|product| product.into()),
            created_by_account_id: value.created_by_account_id,
            created_by_name: value.created_by_name.to_owned(),
            created_at: format!("{:?}", value.created_at),
            done_at: value.done_at.map(|ts| format!("{:?}", ts)),
            done_by_account_id: value.done_by_account_id,
            purchase_id: value.purchase_id,
        }
    }
}

#[derive(Debug, Default, PartialEq, Deserialize, JsonSchema)]
pub struct SaveShoppingListItemDto {
    /// Free text name. Falls back to the product name when empty and `product_id` is given.
    #[serde(default)]
    pub name: Option<String>,
    /// Number of units to buy (defaults to 1, must be >= 1).
    #[serde(default)]
    pub quantity: Option<i32>,
    /// Optional remark (defaults to an empty string).
    #[serde(default)]
    pub note: Option<String>,
    /// Optional reference to a product of the assortment.
    #[serde(default)]
    pub product_id: Option<u64>,
}

#[derive(Debug, Default, PartialEq, Deserialize, JsonSchema)]
pub struct ShoppingListItemDoneDto {
    /// Purchase the item was bought with.
    #[serde(default)]
    pub purchase_id: Option<u64>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ShoppingListQuery {
    /// Which items to return (defaults to `open`).
    #[serde(default)]
    pub state: ShoppingListStateDto,
}

/// Validated fields of a `SaveShoppingListItemDto`.
struct ResolvedItem {
    name: String,
    quantity: i32,
    note: String,
    product: Option<models::Product>,
}

async fn resolve_item(
    state: &mut RequestState,
    form: SaveShoppingListItemDto,
) -> ServiceResult<ResolvedItem> {
    let quantity = form.quantity.unwrap_or(1);
    if quantity < 1 {
        return Err(ServiceError::BadRequest(
            "quantity must be at least 1".to_string(),
        ));
    }

    let name = form
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string);

    if name.is_none() && form.product_id.is_none() {
        return Err(ServiceError::BadRequest(
            "either name or product_id is required".to_string(),
        ));
    }

    let product = match form.product_id {
        Some(product_id) => Some(
            state
                .db
                .get_product_by_id(product_id)
                .await?
                .ok_or(ServiceError::NotFound)?,
        ),
        None => None,
    };

    let name = match (name, product.as_ref()) {
        (Some(name), _) => name,
        (None, Some(product)) => product.name.clone(),
        (None, None) => unreachable!("checked above"),
    };

    Ok(ResolvedItem {
        name,
        quantity,
        note: form.note.unwrap_or_default(),
        product,
    })
}

/// Allows the creator of the item, purchasers and admins.
fn session_require_creator_or_purchaser(
    state: &RequestState,
    item: &models::ShoppingListItem,
) -> ServiceResult<models::Account> {
    let account = state.session_require_login()?;

    if state.session_is_purchaser() || state.session_is_admin() {
        return Ok(account);
    }

    if item
        .created_by_account_id
        .is_some_and(|id| state.session_is_self(id))
    {
        return Ok(account);
    }

    Err(ServiceError::Forbidden)
}

pub async fn list_shopping_list_items(
    mut state: RequestState,
    Query(query): Query<ShoppingListQuery>,
) -> ServiceResult<Json<Vec<ShoppingListItemDto>>> {
    let items = state.db.get_shopping_list_items(query.state.into()).await?;
    Ok(Json(items.iter().map(|i| i.into()).collect()))
}

fn list_shopping_list_items_docs(op: TransformOperation) -> TransformOperation {
    op.description("List shopping list items. `state` defaults to `open`. Open items are ordered oldest first, done items most recently done first.")
        .tag("shopping-list")
        .response::<200, Json<Vec<ShoppingListItemDto>>>()
        .response_with::<400, (), _>(|res| res.description("Invalid state filter!"))
}

pub async fn get_shopping_list_item(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<ShoppingListItemDto>> {
    let item = state.db.get_shopping_list_item_by_id(id).await?;

    if let Some(item) = item {
        return Ok(Json(ShoppingListItemDto::from(&item)));
    }

    Err(ServiceError::NotFound)
}

fn get_shopping_list_item_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get shopping list item by id.")
        .tag("shopping-list")
        .response::<200, Json<ShoppingListItemDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested shopping list item does not exist!")
        })
}

pub async fn create_shopping_list_item(
    mut state: RequestState,
    form: Json<SaveShoppingListItemDto>,
) -> ServiceResult<(StatusCode, Json<ShoppingListItemDto>)> {
    let account = state.session_require_login()?;

    let resolved = resolve_item(&mut state, form.0).await?;

    let item = models::ShoppingListItem {
        id: 0,
        name: resolved.name,
        quantity: resolved.quantity,
        note: resolved.note,
        product: resolved.product,
        created_by_account_id: Some(account.id),
        created_by_name: None,
        created_at: chrono::Utc::now(),
        done_at: None,
        done_by_account_id: None,
        purchase_id: None,
    };

    let item = state.db.create_shopping_list_item(item).await?;
    Ok((StatusCode::CREATED, Json(ShoppingListItemDto::from(&item))))
}

fn create_shopping_list_item_docs(op: TransformOperation) -> TransformOperation {
    op.description("Add an item to the shopping list. Either `name` or `product_id` is required; the creator is taken from the session.")
        .tag("shopping-list")
        .response::<201, Json<ShoppingListItemDto>>()
        .response_with::<400, (), _>(|res| {
            res.description("Missing name/product or quantity below 1!")
        })
        .response_with::<404, (), _>(|res| {
            res.description("The referenced product does not exist!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .security_requirement_scopes("SessionToken", ["basic", "member", "purchaser", "admin"])
}

pub async fn update_shopping_list_item(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SaveShoppingListItemDto>,
) -> ServiceResult<Json<ShoppingListItemDto>> {
    state.session_require_login()?;

    let mut item = state
        .db
        .get_shopping_list_item_by_id(id)
        .await?
        .ok_or(ServiceError::NotFound)?;
    session_require_creator_or_purchaser(&state, &item)?;

    let resolved = resolve_item(&mut state, form.0).await?;
    item.name = resolved.name;
    item.quantity = resolved.quantity;
    item.note = resolved.note;
    item.product = resolved.product;

    let item = state.db.update_shopping_list_item(item).await?;
    Ok(Json(ShoppingListItemDto::from(&item)))
}

fn update_shopping_list_item_docs(op: TransformOperation) -> TransformOperation {
    op.description(
        "Update an open shopping list item. Allowed for the creator, purchasers and admins.",
    )
    .tag("shopping-list")
    .response::<200, Json<ShoppingListItemDto>>()
    .response_with::<400, (), _>(|res| res.description("Missing name/product or quantity below 1!"))
    .response_with::<404, (), _>(|res| {
        res.description("The requested item or the referenced product does not exist!")
    })
    .response_with::<409, (), _>(|res| res.description("The item is already done!"))
    .response_with::<401, (), _>(|res| res.description("Missing login!"))
    .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
    .security_requirement_scopes("SessionToken", ["creator", "purchaser", "admin"])
}

pub async fn delete_shopping_list_item(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<StatusCode> {
    state.session_require_login()?;

    let item = state
        .db
        .get_shopping_list_item_by_id(id)
        .await?
        .ok_or(ServiceError::NotFound)?;
    session_require_creator_or_purchaser(&state, &item)?;

    state.db.delete_shopping_list_item(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_shopping_list_item_docs(op: TransformOperation) -> TransformOperation {
    op.description("Delete a shopping list item (open or done). Allowed for the creator, purchasers and admins.")
        .tag("shopping-list")
        .response_with::<204, (), _>(|res| res.description("The item was successfully deleted!"))
        .response_with::<404, (), _>(|res| {
            res.description("The requested shopping list item does not exist!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["creator", "purchaser", "admin"])
}

pub async fn mark_shopping_list_item_done(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<ShoppingListItemDoneDto>,
) -> ServiceResult<Json<ShoppingListItemDto>> {
    let account = state.session_require_purchaser_or_admin()?;

    let item = state
        .db
        .mark_shopping_list_item_done(id, Some(account.id), form.0.purchase_id)
        .await?;
    Ok(Json(ShoppingListItemDto::from(&item)))
}

fn mark_shopping_list_item_done_docs(op: TransformOperation) -> TransformOperation {
    op.description("Mark an open shopping list item as done, optionally linking the purchase it was bought with.")
        .tag("shopping-list")
        .response::<200, Json<ShoppingListItemDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested item or the referenced purchase does not exist!")
        })
        .response_with::<409, (), _>(|res| res.description("The item is already done!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
}

pub async fn reopen_shopping_list_item(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<ShoppingListItemDto>> {
    state.session_require_purchaser_or_admin()?;

    let item = state.db.reopen_shopping_list_item(id).await?;
    Ok(Json(ShoppingListItemDto::from(&item)))
}

fn reopen_shopping_list_item_docs(op: TransformOperation) -> TransformOperation {
    op.description("Reopen a done shopping list item. Clears `done_at`, `done_by_account_id` and `purchase_id`.")
        .tag("shopping-list")
        .response::<200, Json<ShoppingListItemDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested shopping list item does not exist!")
        })
        .response_with::<409, (), _>(|res| res.description("The item is not done!"))
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["purchaser", "admin"])
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

    fn form(name: &str) -> SaveShoppingListItemDto {
        SaveShoppingListItemDto {
            name: Some(name.to_string()),
            ..Default::default()
        }
    }

    #[sqlx::test]
    async fn permissions_are_enforced(pool: PgPool) {
        let app_state = AppState::from_pool(pool).await;
        let mut db = DatabaseConnection {
            connection: app_state.pool.acquire().await.unwrap(),
        };

        let basic = store_account(&mut db, "basic", Role::Basic).await;
        let other = store_account(&mut db, "other", Role::Member).await;
        let purchaser = store_account(&mut db, "purchaser", Role::Purchaser).await;

        // anonymous users cannot create
        let state = request_state(&app_state, None).await;
        assert_eq!(
            create_shopping_list_item(state, Json(form("Milk")))
                .await
                .err(),
            Some(ServiceError::Unauthorized("Missing login!"))
        );

        // basic user creates own entry
        let state = request_state(&app_state, Some(basic.clone())).await;
        let (status, own) = create_shopping_list_item(state, Json(form("  Milk ")))
            .await
            .unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(own.0.name, "Milk");
        assert_eq!(own.0.quantity, 1);
        assert_eq!(own.0.created_by_account_id, Some(basic.id));
        assert_eq!(own.0.created_by_name.as_deref(), Some("basic"));
        assert_eq!(own.0.done_at, None);

        // another user creates an entry too
        let state = request_state(&app_state, Some(other.clone())).await;
        let (_, foreign) = create_shopping_list_item(state, Json(form("Sugar")))
            .await
            .unwrap();

        // validation
        let state = request_state(&app_state, Some(basic.clone())).await;
        let result = create_shopping_list_item(
            state,
            Json(SaveShoppingListItemDto {
                quantity: Some(0),
                ..form("Milk")
            }),
        )
        .await
        .err();
        assert!(matches!(result, Some(ServiceError::BadRequest(_))));
        let state = request_state(&app_state, Some(basic.clone())).await;
        let result = create_shopping_list_item(state, Json(form("   ")))
            .await
            .err();
        assert!(matches!(result, Some(ServiceError::BadRequest(_))));
        let state = request_state(&app_state, Some(basic.clone())).await;
        let result = create_shopping_list_item(
            state,
            Json(SaveShoppingListItemDto {
                product_id: Some(424242),
                ..Default::default()
            }),
        )
        .await
        .err();
        assert_eq!(result, Some(ServiceError::NotFound));

        // basic user may update own entry but not a foreign one
        let state = request_state(&app_state, Some(basic.clone())).await;
        let updated = update_shopping_list_item(
            state,
            Path(own.0.id),
            Json(SaveShoppingListItemDto {
                quantity: Some(3),
                note: Some("oat".to_string()),
                ..form("Oat milk")
            }),
        )
        .await
        .unwrap();
        assert_eq!(updated.0.name, "Oat milk");
        assert_eq!(updated.0.quantity, 3);
        assert_eq!(updated.0.note, "oat");
        let state = request_state(&app_state, Some(basic.clone())).await;
        assert_eq!(
            update_shopping_list_item(state, Path(foreign.0.id), Json(form("Salt")))
                .await
                .err(),
            Some(ServiceError::Forbidden)
        );

        // basic user cannot mark done or reopen
        let state = request_state(&app_state, Some(basic.clone())).await;
        assert_eq!(
            mark_shopping_list_item_done(state, Path(own.0.id), Json(Default::default()))
                .await
                .err(),
            Some(ServiceError::Forbidden)
        );
        let state = request_state(&app_state, Some(basic.clone())).await;
        assert_eq!(
            reopen_shopping_list_item(state, Path(own.0.id)).await.err(),
            Some(ServiceError::Forbidden)
        );

        // basic user cannot delete a foreign entry, but the own one
        let state = request_state(&app_state, Some(basic.clone())).await;
        assert_eq!(
            delete_shopping_list_item(state, Path(foreign.0.id)).await,
            Err(ServiceError::Forbidden)
        );
        let state = request_state(&app_state, Some(basic.clone())).await;
        assert_eq!(
            delete_shopping_list_item(state, Path(own.0.id)).await,
            Ok(StatusCode::NO_CONTENT)
        );
        let state = request_state(&app_state, Some(basic.clone())).await;
        assert_eq!(
            delete_shopping_list_item(state, Path(own.0.id)).await,
            Err(ServiceError::NotFound)
        );

        // purchaser can mark the foreign entry done, update is then a conflict
        let state = request_state(&app_state, Some(purchaser.clone())).await;
        let done =
            mark_shopping_list_item_done(state, Path(foreign.0.id), Json(Default::default()))
                .await
                .unwrap();
        assert!(done.0.done_at.is_some());
        assert_eq!(done.0.done_by_account_id, Some(purchaser.id));
        let state = request_state(&app_state, Some(purchaser.clone())).await;
        assert!(matches!(
            update_shopping_list_item(state, Path(foreign.0.id), Json(form("Salt")))
                .await
                .err(),
            Some(ServiceError::Conflict(_))
        ));
        let state = request_state(&app_state, Some(purchaser.clone())).await;
        assert!(matches!(
            mark_shopping_list_item_done(state, Path(foreign.0.id), Json(Default::default()))
                .await
                .err(),
            Some(ServiceError::Conflict(_))
        ));

        // unknown purchase id on done
        let state = request_state(&app_state, Some(purchaser.clone())).await;
        let reopened = reopen_shopping_list_item(state, Path(foreign.0.id))
            .await
            .unwrap();
        assert_eq!(reopened.0.done_at, None);
        let state = request_state(&app_state, Some(purchaser.clone())).await;
        assert_eq!(
            mark_shopping_list_item_done(
                state,
                Path(foreign.0.id),
                Json(ShoppingListItemDoneDto {
                    purchase_id: Some(424242),
                }),
            )
            .await
            .err(),
            Some(ServiceError::NotFound)
        );

        // purchaser can delete foreign entries; the list is public
        let state = request_state(&app_state, Some(purchaser.clone())).await;
        assert_eq!(
            delete_shopping_list_item(state, Path(foreign.0.id)).await,
            Ok(StatusCode::NO_CONTENT)
        );
        let state = request_state(&app_state, None).await;
        let list = list_shopping_list_items(state, Query(ShoppingListQuery::default()))
            .await
            .unwrap();
        assert!(list.0.is_empty());
    }
}
