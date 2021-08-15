use async_graphql::{Context, Object};
use uuid::Uuid;

use crate::repo::{self, AccountOutput, CategoryOutput, ProductOutput, SearchElement};
use crate::{identity_service::Identity, model::ServiceResult};

use super::get_conn_from_ctx;

pub struct Query;

#[Object]
impl Query {
    async fn get_self(&self, ctx: &Context<'_>) -> ServiceResult<AccountOutput> {
        let identity = ctx.data::<Identity>()?;
        repo::get_me(identity)
    }

    async fn get_accounts(
        &self,
        ctx: &Context<'_>,
        search: Option<String>,
    ) -> ServiceResult<Vec<SearchElement<AccountOutput>>> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_accounts(conn, identity, search.as_deref())
    }

    async fn get_account(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<AccountOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_account(conn, identity, id)
    }

    #[graphql(entity)]
    async fn find_account_by_id(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> ServiceResult<AccountOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_account(conn, identity, id)
    }

    async fn get_categories(
        &self,
        ctx: &Context<'_>,
        search: Option<String>,
    ) -> ServiceResult<Vec<SearchElement<CategoryOutput>>> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_categories(conn, identity, search.as_deref())
    }

    async fn get_category(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<CategoryOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_category(conn, identity, id)
    }

    #[graphql(entity)]
    async fn find_category_by_id(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> ServiceResult<CategoryOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_category(conn, identity, id)
    }

    async fn get_products(
        &self,
        ctx: &Context<'_>,
        search: Option<String>,
    ) -> ServiceResult<Vec<SearchElement<ProductOutput>>> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_products(conn, identity, search.as_deref())
    }

    async fn get_product(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<ProductOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_product(conn, identity, id)
    }

    #[graphql(entity)]
    async fn find_product_by_id(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> ServiceResult<ProductOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_product(conn, identity, id)
    }
}
