use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use log::error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::request_state::RequestState;
use crate::{models, wallet};

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/account-status/:id",
            get_with(get_account_status, get_account_status_docs)
                .put_with(update_account_status, update_account_status_docs)
                .delete_with(delete_account_status, delete_account_status_docs),
        )
        .api_route(
            "/account-status",
            get_with(list_account_status, list_account_status_docs)
                .post_with(create_account_status, create_account_status_docs),
        )
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct AccountStatusDto {
    pub id: u64,
    pub name: String,
    pub priority: u64,
}

impl From<&models::AccountStatus> for AccountStatusDto {
    fn from(value: &models::AccountStatus) -> Self {
        Self {
            id: value.id.to_owned(),
            name: value.name.to_owned(),
            priority: value.priority.to_owned(),
        }
    }
}

async fn list_account_status(
    mut state: RequestState,
) -> ServiceResult<Json<Vec<AccountStatusDto>>> {
    state.session_require_admin()?;

    let accounts = state.db.get_all_account_status().await?;
    Ok(Json(accounts.iter().map(|a| a.into()).collect()))
}

fn list_account_status_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all account status.")
        .tag("account_status")
        .response::<200, Json<Vec<AccountStatusDto>>>()
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}

pub async fn get_account_status(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<AccountStatusDto>> {
    state.session_require_admin_or_self(id)?;

    let account_status = state.db.get_account_status_by_id(id).await?;

    if let Some(account) = account_status {
        return Ok(Json(AccountStatusDto::from(&account)));
    }

    Err(ServiceError::NotFound)
}

fn get_account_status_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get an account status by id.")
        .tag("account_status")
        .response::<200, Json<AccountStatusDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested account status does not exist!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SaveAccountStatusDto {
    pub name: String,
    pub priority: u64,
}

async fn create_account_status(
    mut state: RequestState,
    form: Json<SaveAccountStatusDto>,
) -> ServiceResult<Json<AccountStatusDto>> {
    state.session_require_admin()?;

    let form = form.0;

    let account_status = models::AccountStatus {
        id: 0,
        name: form.name,
        priority: form.priority,
    };

    let account_status = state.db.store_account_status(account_status).await?;
    Ok(Json(AccountStatusDto::from(&account_status)))
}

fn create_account_status_docs(op: TransformOperation) -> TransformOperation {
    op.description("Create a new account status.")
        .tag("account_status")
        .response::<200, Json<AccountStatusDto>>()
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}

async fn update_account_status(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SaveAccountStatusDto>,
) -> ServiceResult<Json<AccountStatusDto>> {
    state.session_require_admin_or_self(id)?;

    let form = form.0;
    let account_status = state.db.get_account_status_by_id(id).await?;

    if let Some(mut account) = account_status {
        account.name = form.name;
        account.priority = form.priority;

        let account_status = state.db.store_account_status(account).await?;

        tokio::task::spawn(async move {
            if let Err(e) = wallet::send_update_notification(&mut state.db, id).await {
                error!("Could not send apns update! {:?}", e)
            }
        });

        return Ok(Json(AccountStatusDto::from(&account_status)));
    }

    Err(ServiceError::NotFound)
}

fn update_account_status_docs(op: TransformOperation) -> TransformOperation {
    op.description("Update an existing account status.")
        .tag("account_status")
        .response::<200, Json<AccountStatusDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested account status does not exist!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}

async fn delete_account_status(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<StatusCode> {
    state.session_require_admin_or_self(id)?;

    state.db.delete_account_status(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_account_status_docs(op: TransformOperation) -> TransformOperation {
    op.description("Delete an existing account status.")
        .tag("account_status")
        .response_with::<204, (), _>(|res| {
            res.description("The account status was successfully deleted!")
        })
        .response_with::<404, (), _>(|res| {
            res.description("The requested account status does not exist!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin", "self"])
}
