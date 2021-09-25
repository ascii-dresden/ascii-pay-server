use std::ops::DerefMut;

use async_graphql::{Context, Upload};
use uuid::Uuid;

use crate::identity_service::Identity;
use crate::repo::{
    self, AccountAccessTokenOutput, AccountInput, AccountOutput, CategoryInput, CategoryOutput,
    LoginInput, LoginOutput, PaymentInput, PaymentOutput, ProductInput, ProductOutput,
};
use crate::utils::ServiceResult;

use super::{get_database_conn_from_ctx, get_redis_conn_from_ctx};

pub struct Mutation;

#[Object]
impl Mutation {
    async fn login(&self, ctx: &Context<'_>, input: LoginInput) -> ServiceResult<LoginOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let mut redis_conn = get_redis_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::login(database_conn, redis_conn.deref_mut(), identity, input)
    }

    async fn logout(&self, ctx: &Context<'_>) -> ServiceResult<String> {
        let mut redis_conn = get_redis_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::logout(redis_conn.deref_mut(), identity)?;
        Ok("ok".to_string())
    }

    async fn create_account(
        &self,
        ctx: &Context<'_>,
        input: AccountInput,
    ) -> ServiceResult<AccountOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::create_account(database_conn, identity, input)
    }

    async fn update_account(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: AccountInput,
    ) -> ServiceResult<AccountOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::update_account(database_conn, identity, id, input)
    }

    async fn delete_account(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<String> {
        let conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::delete_account(conn, identity, id)?;
        Ok("ok".to_string())
    }

    async fn delete_account_nfc_card(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<String> {
        let conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::authenticate_nfc_delete_card(conn, identity, id)?;
        Ok("ok".to_string())
    }

    async fn get_account_access_token(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> ServiceResult<AccountAccessTokenOutput> {
        let conn = &get_database_conn_from_ctx(ctx)?;
        let mut redis_conn = get_redis_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::authenticate_account(conn, redis_conn.deref_mut(), identity, id)
    }

    async fn create_category(
        &self,
        ctx: &Context<'_>,
        input: CategoryInput,
    ) -> ServiceResult<CategoryOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::create_category(database_conn, identity, input)
    }

    async fn update_category(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: CategoryInput,
    ) -> ServiceResult<CategoryOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::update_category(database_conn, identity, id, input)
    }

    async fn delete_category(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<String> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::delete_category(database_conn, identity, id)?;
        Ok("ok".to_string())
    }

    async fn create_product(
        &self,
        ctx: &Context<'_>,
        input: ProductInput,
    ) -> ServiceResult<ProductOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::create_product(database_conn, identity, input)
    }

    async fn update_product(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: ProductInput,
    ) -> ServiceResult<ProductOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::update_product(database_conn, identity, id, input)
    }

    async fn delete_product(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<String> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::delete_product(database_conn, identity, id)?;
        Ok("ok".to_string())
    }

    async fn transaction(
        &self,
        ctx: &Context<'_>,
        input: PaymentInput,
    ) -> ServiceResult<PaymentOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let mut redis_conn = get_redis_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::transaction_payment(database_conn, redis_conn.deref_mut(), identity, input)
    }

    async fn remove_product_image(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<String> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;

        repo::remove_product_image(database_conn, identity, id)?;
        Ok("ok".to_string())
    }

    async fn set_product_image(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        image: Upload,
    ) -> ServiceResult<String> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;

        let mut upload_data = image.value(ctx).unwrap();
        repo::set_product_image(
            database_conn,
            identity,
            id,
            &upload_data.filename,
            upload_data.content_type.as_deref(),
            &mut upload_data.content,
        )?;
        Ok("ok".to_string())
    }
}
