mod model;
mod mutation;
mod query;
mod subscription;

use std::sync::Arc;

use actix_web::{guard, web, HttpRequest, HttpResponse};
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    Context, Schema,
};
use async_graphql_actix_web::{Request, Response, WSSubscription};

use crate::core::{Pool, ServiceResult};

use self::{mutation::Mutation, query::Query, subscription::Subscription};

pub type AppSchema = Schema<Query, Mutation, Subscription>;

pub fn get_conn_from_ctx(
    ctx: &Context<'_>,
) -> ServiceResult<r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>> {
    Ok(ctx.data::<Arc<Pool>>()?.get()?)
}

pub fn create_schema_with_context(pool: Pool) -> AppSchema {
    let arc_pool = Arc::new(pool);

    Schema::build(Query, Mutation, Subscription)
        .data(arc_pool)
        .finish()
}

async fn index(schema: web::Data<AppSchema>, req: Request) -> Response {
    let query = req.into_inner();
    schema.execute(query).await.into()
}

async fn index_ws(
    schema: web::Data<AppSchema>,
    req: HttpRequest,
    payload: web::Payload,
) -> ServiceResult<HttpResponse> {
    Ok(WSSubscription::start(
        Schema::clone(&*schema),
        &req,
        payload,
    )?)
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
            .route(
                web::get()
                    .guard(guard::Header("upgrade", "websocket"))
                    .to(self::index_ws),
            )
            .route(web::get().to(self::index_playground)),
    );
}
