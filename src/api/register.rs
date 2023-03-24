use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::models;
use crate::request_state::RequestState;

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/register/:id",
            get_with(get_register_history, get_register_history_docs)
                .put_with(update_register_history, update_register_history_docs)
                .delete_with(delete_register_history, delete_register_history_docs),
        )
        .api_route(
            "/register",
            get_with(list_register_histories, list_register_histories_docs)
                .post_with(create_register_history, create_register_history_docs),
        )
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct RegisterHistoryDto {
    pub id: u64,
    pub timestamp: String,
    pub source_register: RegisterHistoryStateDto,
    pub target_register: RegisterHistoryStateDto,
    pub envelope_register: RegisterHistoryStateDto,
}

impl From<&models::RegisterHistory> for RegisterHistoryDto {
    fn from(value: &models::RegisterHistory) -> Self {
        Self {
            id: value.id.to_owned(),
            timestamp: format!("{:?}", value.timestamp),
            target_register: value.target_register.into(),
            source_register: value.source_register.into(),
            envelope_register: value.envelope_register.into(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy, JsonSchema)]
pub struct RegisterHistoryStateDto {
    coin200: i32,
    coin100: i32,
    coin50: i32,
    coin20: i32,
    coin10: i32,
    coin5: i32,
    coin2: i32,
    coin1: i32,
    note100: i32,
    note50: i32,
    note20: i32,
    note10: i32,
    note5: i32,
}

impl From<RegisterHistoryStateDto> for models::RegisterHistoryState {
    fn from(value: RegisterHistoryStateDto) -> Self {
        models::RegisterHistoryState {
            coin200: value.coin200,
            coin100: value.coin100,
            coin50: value.coin50,
            coin20: value.coin20,
            coin10: value.coin10,
            coin5: value.coin5,
            coin2: value.coin2,
            coin1: value.coin1,
            note100: value.note100,
            note50: value.note50,
            note20: value.note20,
            note10: value.note10,
            note5: value.note5,
        }
    }
}

impl From<models::RegisterHistoryState> for RegisterHistoryStateDto {
    fn from(value: models::RegisterHistoryState) -> Self {
        RegisterHistoryStateDto {
            coin200: value.coin200,
            coin100: value.coin100,
            coin50: value.coin50,
            coin20: value.coin20,
            coin10: value.coin10,
            coin5: value.coin5,
            coin2: value.coin2,
            coin1: value.coin1,
            note100: value.note100,
            note50: value.note50,
            note20: value.note20,
            note10: value.note10,
            note5: value.note5,
        }
    }
}

pub async fn list_register_histories(
    mut state: RequestState,
) -> ServiceResult<Json<Vec<RegisterHistoryDto>>> {
    let register_histories = state.db.get_all_register_histories().await?;
    Ok(Json(register_histories.iter().map(|p| p.into()).collect()))
}

fn list_register_histories_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all register histories.")
        .tag("register_histories")
        .response::<200, Json<Vec<RegisterHistoryDto>>>()
}

pub async fn get_register_history(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<Json<RegisterHistoryDto>> {
    let register_history = state.db.get_register_history_by_id(id).await?;

    if let Some(register_history) = register_history {
        return Ok(Json(RegisterHistoryDto::from(&register_history)));
    }

    Err(ServiceError::NotFound)
}

fn get_register_history_docs(op: TransformOperation) -> TransformOperation {
    op.description("Get a register history by id.")
        .tag("register_histories")
        .response::<200, Json<RegisterHistoryDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested register_history does not exist!")
        })
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct SaveRegisterHistoryDto {
    pub source_register: RegisterHistoryStateDto,
    pub target_register: RegisterHistoryStateDto,
    pub envelope_register: RegisterHistoryStateDto,
}

async fn create_register_history(
    mut state: RequestState,
    form: Json<SaveRegisterHistoryDto>,
) -> ServiceResult<Json<RegisterHistoryDto>> {
    let form = form.0;

    let register_history = models::RegisterHistory {
        id: 0,
        timestamp: Utc::now(),
        source_register: form.source_register.into(),
        target_register: form.target_register.into(),
        envelope_register: form.envelope_register.into(),
    };

    let register_history = state.db.store_register_history(register_history).await?;
    Ok(Json(RegisterHistoryDto::from(&register_history)))
}

fn create_register_history_docs(op: TransformOperation) -> TransformOperation {
    op.description("Create a new register history.")
        .tag("register_histories")
        .response::<200, Json<RegisterHistoryDto>>()
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}

async fn update_register_history(
    mut state: RequestState,
    Path(id): Path<u64>,
    form: Json<SaveRegisterHistoryDto>,
) -> ServiceResult<Json<RegisterHistoryDto>> {
    state.session_require_admin()?;

    let form = form.0;
    let register_history = state.db.get_register_history_by_id(id).await?;

    if let Some(mut register_history) = register_history {
        register_history.source_register = form.source_register.into();
        register_history.target_register = form.target_register.into();
        register_history.envelope_register = form.envelope_register.into();

        let register_history = state.db.store_register_history(register_history).await?;
        return Ok(Json(RegisterHistoryDto::from(&register_history)));
    }

    Err(ServiceError::NotFound)
}

fn update_register_history_docs(op: TransformOperation) -> TransformOperation {
    op.description("Update an existing register history.")
        .tag("register_histories")
        .response::<200, Json<RegisterHistoryDto>>()
        .response_with::<404, (), _>(|res| {
            res.description("The requested register history does not exist!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}

async fn delete_register_history(
    mut state: RequestState,
    Path(id): Path<u64>,
) -> ServiceResult<StatusCode> {
    state.session_require_admin()?;

    state.db.delete_register_history(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn delete_register_history_docs(op: TransformOperation) -> TransformOperation {
    op.description("Delete an existing register history.")
        .tag("register_histories")
        .response_with::<204, (), _>(|res| {
            res.description("The register history was successfully deleted!")
        })
        .response_with::<404, (), _>(|res| {
            res.description("The requested register_history does not exist!")
        })
        .response_with::<401, (), _>(|res| res.description("Missing login!"))
        .response_with::<403, (), _>(|res| res.description("Missing permissions!"))
        .security_requirement_scopes("SessionToken", ["admin"])
}
