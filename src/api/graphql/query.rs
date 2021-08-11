use std::convert::TryFrom;

use async_graphql::{Context, Object, ID};
use uuid::Uuid;

use crate::core;

use super::{get_conn_from_ctx, model::Category, model::Product};

pub struct Query;

#[Object]
impl Query {
    async fn get_categories(&self, ctx: &Context<'_>) -> Vec<Category> {
        let conn = &get_conn_from_ctx(ctx).unwrap();
        core::Category::all(conn)
            .expect("Can't get categories")
            .iter()
            .map(Category::from)
            .collect()
    }

    async fn get_category(&self, ctx: &Context<'_>, id: ID) -> Option<Category> {
        let conn = &get_conn_from_ctx(ctx).unwrap();

        let uuid = Uuid::try_from(id).ok()?;
        core::Category::get(conn, &uuid)
            .as_ref()
            .map(Category::from)
            .ok()
    }

    #[graphql(entity)]
    async fn find_category_by_id(&self, ctx: &Context<'_>, id: ID) -> Option<Category> {
        let conn = &get_conn_from_ctx(ctx).unwrap();

        let uuid = Uuid::try_from(id).ok()?;
        core::Category::get(conn, &uuid)
            .as_ref()
            .map(Category::from)
            .ok()
    }

    async fn get_products(&self, ctx: &Context<'_>) -> Vec<Product> {
        let conn = &get_conn_from_ctx(ctx).unwrap();
        core::Product::all(conn)
            .expect("Can't get planets")
            .iter()
            .map(Product::from)
            .collect()
    }

    async fn get_product(&self, ctx: &Context<'_>, id: ID) -> Option<Product> {
        let conn = &get_conn_from_ctx(ctx).unwrap();

        let uuid = Uuid::try_from(id).ok()?;
        core::Product::get(conn, &uuid)
            .as_ref()
            .map(Product::from)
            .ok()
    }

    #[graphql(entity)]
    async fn find_product_by_id(&self, ctx: &Context<'_>, id: ID) -> Option<Product> {
        let conn = &get_conn_from_ctx(ctx).unwrap();

        let uuid = Uuid::try_from(id).ok()?;
        core::Product::get(conn, &uuid)
            .as_ref()
            .map(Product::from)
            .ok()
    }
}
