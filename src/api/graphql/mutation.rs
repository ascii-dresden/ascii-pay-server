use async_graphql::Context;
use uuid::Uuid;

use crate::identity_service::Identity;
use crate::model::ServiceResult;
use crate::repo::{
    self, AccountInput, AccountOutput, CategoryInput, CategoryOutput, LoginInput, LoginOutput,
    ProductInput, ProductOutput,
};

use super::get_conn_from_ctx;

pub struct Mutation;

#[Object]
impl Mutation {
    async fn login(&self, ctx: &Context<'_>, input: LoginInput) -> ServiceResult<LoginOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::login(conn, identity, input)
    }

    async fn logout(&self, ctx: &Context<'_>) -> ServiceResult<String> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::logout(conn, identity)?;
        Ok("ok".to_string())
    }

    async fn create_account(
        &self,
        ctx: &Context<'_>,
        input: AccountInput,
    ) -> ServiceResult<AccountOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::create_account(conn, identity, input)
    }

    async fn update_account(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: AccountInput,
    ) -> ServiceResult<AccountOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::update_account(conn, identity, id, input)
    }

    async fn delete_account(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<String> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::delete_account(conn, identity, id)?;
        Ok("ok".to_string())
    }

    async fn create_category(
        &self,
        ctx: &Context<'_>,
        input: CategoryInput,
    ) -> ServiceResult<CategoryOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::create_category(conn, identity, input)
    }

    async fn update_category(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: CategoryInput,
    ) -> ServiceResult<CategoryOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::update_category(conn, identity, id, input)
    }

    async fn delete_category(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<String> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::delete_category(conn, identity, id)?;
        Ok("ok".to_string())
    }

    async fn create_product(
        &self,
        ctx: &Context<'_>,
        input: ProductInput,
    ) -> ServiceResult<ProductOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::create_product(conn, identity, input)
    }

    async fn update_product(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: ProductInput,
    ) -> ServiceResult<ProductOutput> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::update_product(conn, identity, id, input)
    }

    async fn delete_product(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<String> {
        let conn = &get_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::delete_product(conn, identity, id)?;
        Ok("ok".to_string())
    }
}
