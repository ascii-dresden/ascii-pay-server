use std::ops::Add;
use std::time::{Duration, Instant};

use aide::axum::routing::post_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use argon2rs::verifier::Encoded;
use axum::Json;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::database::AppState;
use crate::error::{ServiceError, ServiceResult};
use crate::{models, RequestState};

pub fn router(app_state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/auth/password",
            post_with(auth_password_based, auth_password_based_docs),
        )
        .with_state(app_state)
}

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
pub struct AuthTokenDto {
    pub token: String,
}

#[derive(Debug, PartialEq, Deserialize, JsonSchema)]
pub struct AuthPasswordBasedDto {
    pub username: String,
    pub password: String,
}

async fn auth_password_based(
    state: RequestState,
    form: Json<AuthPasswordBasedDto>,
) -> ServiceResult<Json<AuthTokenDto>> {
    let form = form.0;
    let account = state.db.get_account_by_auth_method(Vec::new()).await?;

    if let Some(account) = account {
        for auth_method in account.auth_methods.iter() {
            if let models::AuthMethod::PasswordBased(password_based) = auth_method {
                if password_hash_verify(&password_based.password_hash, &form.password)?
                    && password_based.username == form.username
                {
                    let token = state
                        .db
                        .create_session_token(
                            account.id,
                            models::AuthMethodType::PasswordBased,
                            Instant::now().add(Duration::from_secs(30 * 60)),
                            false,
                        )
                        .await?;

                    return Ok(Json(AuthTokenDto { token }));
                }
            }
        }

        return Err(ServiceError::NotFound);
    }

    Err(ServiceError::NotFound)
}

fn auth_password_based_docs(op: TransformOperation) -> TransformOperation {
    op.description("Login with username and password.")
        .response::<200, Json<AuthTokenDto>>()
        .response::<404, ()>()
        .response::<500, ()>()
}

fn password_hash_verify(hash: &[u8], password: &str) -> ServiceResult<bool> {
    if let Ok(enc) = Encoded::from_u8(hash) {
        return Ok(enc.verify(password.as_bytes()));
    }

    Ok(false)
}
