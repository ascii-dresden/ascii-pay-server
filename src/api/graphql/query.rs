use std::convert::TryFrom;

use async_graphql::{Context, Object, ID};
use uuid::Uuid;

use crate::{
    core::{self, Permission, ServiceResult},
    identity_service::{Identity, IdentityRequire},
};

use super::{
    get_conn_from_ctx,
    model::Product,
    model::{Account, Category},
};

pub struct Query;

#[Object]
impl Query {
    async fn get_self(&self, ctx: &Context<'_>) -> ServiceResult<Option<Account>> {
        let identity = ctx.data::<Identity>()?;
        Ok(identity.get_account()?.as_ref().map(Account::from))
    }

    async fn get_categories(&self, ctx: &Context<'_>) -> ServiceResult<Vec<Category>> {
        let identity = ctx.data::<Identity>()?;
        identity.require_account_or_cert(Permission::MEMBER)?;

        let conn = &get_conn_from_ctx(ctx)?;
        Ok(core::Category::all(conn)?
            .iter()
            .map(Category::from)
            .collect())
    }

    async fn get_category(&self, ctx: &Context<'_>, id: ID) -> ServiceResult<Category> {
        let identity = ctx.data::<Identity>()?;
        identity.require_account_or_cert(Permission::MEMBER)?;

        let conn = &get_conn_from_ctx(ctx)?;

        let uuid = Uuid::try_from(id)?;
        Ok(Category::from(&core::Category::get(conn, &uuid)?))
    }

    #[graphql(entity)]
    async fn find_category_by_id(&self, ctx: &Context<'_>, id: ID) -> ServiceResult<Category> {
        let identity = ctx.data::<Identity>()?;
        identity.require_account_or_cert(Permission::MEMBER)?;

        let conn = &get_conn_from_ctx(ctx)?;

        let uuid = Uuid::try_from(id)?;
        Ok(Category::from(&core::Category::get(conn, &uuid)?))
    }

    async fn get_products(&self, ctx: &Context<'_>) -> ServiceResult<Vec<Product>> {
        let identity = ctx.data::<Identity>()?;
        identity.require_account_or_cert(Permission::MEMBER)?;

        let conn = &get_conn_from_ctx(ctx)?;
        Ok(core::Product::all(conn)?
            .iter()
            .map(Product::from)
            .collect())
    }

    async fn get_product(&self, ctx: &Context<'_>, id: ID) -> ServiceResult<Product> {
        let identity = ctx.data::<Identity>()?;
        identity.require_account_or_cert(Permission::MEMBER)?;

        let conn = &get_conn_from_ctx(ctx)?;

        let uuid = Uuid::try_from(id)?;
        Ok(Product::from(&core::Product::get(conn, &uuid)?))
    }

    #[graphql(entity)]
    async fn find_product_by_id(&self, ctx: &Context<'_>, id: ID) -> ServiceResult<Product> {
        let identity = ctx.data::<Identity>()?;
        identity.require_account_or_cert(Permission::MEMBER)?;

        let conn = &get_conn_from_ctx(ctx)?;

        let uuid = Uuid::try_from(id)?;
        Ok(Product::from(&core::Product::get(conn, &uuid)?))
    }
}
