use crate::identity_service::Identity;
use crate::model::{Pool, ServiceResult};
use crate::repo::{self, IdentificationInput};

use actix_web::{web, HttpResponse};

/// POST route for `/api/v1/identify`
pub async fn post_identify(
    pool: web::Data<Pool>,
    identity: Identity,
    input: web::Json<IdentificationInput>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let result = repo::identify(conn, &identity, input.into_inner())?;
    Ok(HttpResponse::Ok().json(&result))
}
