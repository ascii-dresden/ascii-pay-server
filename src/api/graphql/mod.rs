mod model;
mod mutation;
mod query;

use std::sync::Arc;

use actix_web::{web, HttpResponse};
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    Context, EmptySubscription, Schema,
};
use async_graphql_actix_web::{Request, Response};

use crate::{
    core::{Pool, ServiceResult},
    identity_service::Identity,
};

use self::{mutation::Mutation, query::Query};

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

pub fn get_conn_from_ctx(
    ctx: &Context<'_>,
) -> ServiceResult<r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>> {
    Ok(ctx.data::<Arc<Pool>>()?.get()?)
}

pub fn create_schema_with_context(pool: Pool) -> AppSchema {
    let arc_pool = Arc::new(pool);

    Schema::build(Query, Mutation, EmptySubscription)
        .data(arc_pool)
        .finish()
}

async fn index(schema: web::Data<AppSchema>, identity: Identity, req: Request) -> Response {
    let mut query = req.into_inner();
    query = query.data(identity);
    schema.execute(query).await.into()
}

async fn index_playground() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(
            GraphQLPlaygroundConfig::new("/api/v1/graphql")
                .subscription_endpoint("/api/v1/graphql"),
        ))
}

pub fn init(config: &mut web::ServiceConfig) {
    config.service(
        web::resource("")
            .route(web::post().to(self::index))
            .route(web::get().to(self::index_playground)),
    );
}
