use async_graphql::{Context, Object};
use uuid::Uuid;

use crate::repo::{self, AccountOutput, CategoryOutput, ProductOutput, SearchElement};
use crate::{identity_service::Identity, utils::ServiceResult};

use super::get_database_conn_from_ctx;

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
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_accounts(database_conn, identity, search.as_deref())
    }

    async fn get_account(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<AccountOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_account(database_conn, identity, id)
    }

    #[graphql(entity)]
    async fn find_account_by_id(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> ServiceResult<AccountOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_account(database_conn, identity, id)
    }

    async fn get_categories(
        &self,
        ctx: &Context<'_>,
        search: Option<String>,
    ) -> ServiceResult<Vec<SearchElement<CategoryOutput>>> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_categories(database_conn, identity, search.as_deref())
    }

    async fn get_category(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<CategoryOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_category(database_conn, identity, id)
    }

    #[graphql(entity)]
    async fn find_category_by_id(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> ServiceResult<CategoryOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_category(database_conn, identity, id)
    }

    async fn get_products(
        &self,
        ctx: &Context<'_>,
        search: Option<String>,
    ) -> ServiceResult<Vec<SearchElement<ProductOutput>>> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_products(database_conn, identity, search.as_deref())
    }

    async fn get_product(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<ProductOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_product(database_conn, identity, id)
    }

    #[graphql(entity)]
    async fn find_product_by_id(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> ServiceResult<ProductOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::get_product(database_conn, identity, id)
    }
}
