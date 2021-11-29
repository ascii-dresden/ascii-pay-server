mod mutation;
mod query;

use std::sync::Arc;

use actix_web::{web, HttpResponse};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::{EmptySubscription, Schema};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};

use crate::identity_service::Identity;
use crate::utils::{DatabasePool, RedisPool};

use self::{mutation::Mutation, query::Query};

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

pub fn print_grahpql_schema() {
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
    println!("{}", schema.sdl());
}

pub fn create_schema_with_context(databse_pool: DatabasePool, redis_pool: RedisPool) -> AppSchema {
    let arc_database_pool = Arc::new(databse_pool);
    let arc_redis_pool = Arc::new(redis_pool);

    Schema::build(Query, Mutation, EmptySubscription)
        .data(arc_database_pool)
        .data(arc_redis_pool)
        .finish()
}

async fn index(
    schema: web::Data<AppSchema>,
    identity: Identity,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut query = req.into_inner();
    query = query.data(identity);
    schema.execute(query).await.into()
}

async fn index_playground() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(GraphQLPlaygroundConfig::new(
            "/api/v1/graphql",
        )))
}

pub fn init(config: &mut web::ServiceConfig) {
    config.service(
        web::resource("")
            .route(web::post().to(self::index))
            .route(web::get().to(self::index_playground)),
    );
}
