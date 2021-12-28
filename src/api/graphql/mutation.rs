use std::ops::Deref;
use std::sync::Arc;

use async_graphql::Context;
use uuid::Uuid;

use crate::identity_service::Identity;
use crate::model::session::Session;
use crate::repo::{
    self, AccountAccessTokenOutput, AccountCreateInput, AccountOutput, AccountUpdateInput,
    LoginInput, LoginOutput, PaymentInput, PaymentOutput,
};
use crate::utils::{DatabasePool, RedisPool, ServiceResult};

pub struct Mutation;

#[Object]
impl Mutation {
    async fn login(
        &self,
        ctx: &Context<'_>,
        username: Option<String>,
        password: Option<String>,
        account_access_token: Option<Session>,
    ) -> ServiceResult<LoginOutput> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let redis_pool = ctx.data::<Arc<RedisPool>>()?;
        let identity = ctx.data::<Identity>()?;

        repo::login(
            database_pool.deref(),
            redis_pool.deref(),
            identity,
            LoginInput {
                username,
                password,
                account_access_token,
            },
        )
        .await
    }

    async fn logout(&self, ctx: &Context<'_>) -> ServiceResult<String> {
        let redis_pool = ctx.data::<Arc<RedisPool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::logout(redis_pool.deref(), identity).await?;
        Ok("ok".to_string())
    }

    async fn create_account(
        &self,
        ctx: &Context<'_>,
        input: AccountCreateInput,
    ) -> ServiceResult<AccountOutput> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::create_account(database_pool.deref(), identity, input).await
    }

    async fn update_account(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: AccountUpdateInput,
    ) -> ServiceResult<AccountOutput> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::update_account(database_pool.deref(), identity, id, input).await
    }

    async fn delete_account(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<String> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::delete_account(database_pool.deref(), identity, id)?;
        Ok("ok".to_string())
    }

    async fn delete_account_nfc_card(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        card_id: String,
    ) -> ServiceResult<String> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::authenticate_nfc_delete_card(database_pool.deref(), identity, id, &card_id).await?;
        Ok("ok".to_string())
    }

    async fn get_account_access_token(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> ServiceResult<AccountAccessTokenOutput> {
        let redis_pool = ctx.data::<Arc<RedisPool>>()?;
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;

        repo::authenticate_account(database_pool.deref(), redis_pool.deref(), identity, id).await
    }

    async fn transaction(
        &self,
        ctx: &Context<'_>,
        input: PaymentInput,
    ) -> ServiceResult<PaymentOutput> {
        let redis_pool = ctx.data::<Arc<RedisPool>>()?;
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;

        repo::transaction_payment(database_pool.deref(), redis_pool.deref(), identity, input).await
    }

    async fn update_products(&self, ctx: &Context<'_>) -> ServiceResult<String> {
        let identity = ctx.data::<Identity>()?;
        repo::update_products(identity)?;
        Ok("ok".to_string())
    }
}
