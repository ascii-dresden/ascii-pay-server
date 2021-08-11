use async_graphql::{Context, Object};
use chrono::NaiveDateTime;

use crate::core::{self, ServiceResult};

use super::{
    get_conn_from_ctx,
    model::{Category, CategoryCreateInput},
};

pub struct Mutation;

#[Object]
impl Mutation {
    async fn create_category(
        &self,
        ctx: &Context<'_>,
        category: CategoryCreateInput,
    ) -> ServiceResult<Category> {
        let conn = &get_conn_from_ctx(ctx)?;
        let mut entity = core::Category::create(conn, &category.name)?;

        if let Some(price) = category.price {
            entity.add_price(conn, NaiveDateTime::from_timestamp(0, 0), price)?;
        }

        Ok(Category::from(&entity))
    }
}
