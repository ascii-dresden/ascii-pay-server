use std::ops::DerefMut;

use async_graphql::Context;
use uuid::Uuid;

use crate::identity_service::Identity;
use crate::model::session::Session;
use crate::repo::{
    self, AccountAccessTokenOutput, AccountCreateInput, AccountOutput, AccountUpdateInput,
    LoginInput, LoginOutput, PaymentInput, PaymentOutput,
};
use crate::utils::ServiceResult;

use super::{get_database_conn_from_ctx, get_redis_conn_from_ctx};

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
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let mut redis_conn = get_redis_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::login(
            database_conn,
            redis_conn.deref_mut(),
            identity,
            LoginInput {
                username,
                password,
                account_access_token,
            },
        )
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
        input: AccountCreateInput,
    ) -> ServiceResult<AccountOutput> {
        let database_conn = &get_database_conn_from_ctx(ctx)?;
        let identity = ctx.data::<Identity>()?;
        repo::create_account(database_conn, identity, input)
    }

    async fn update_account(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: AccountUpdateInput,
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

    async fn update_products(&self, ctx: &Context<'_>) -> ServiceResult<String> {
        let identity = ctx.data::<Identity>()?;
        repo::update_products(identity)?;
        Ok("ok".to_string())
    }
}
