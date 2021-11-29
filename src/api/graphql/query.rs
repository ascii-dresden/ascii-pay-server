use std::ops::Deref;
use std::sync::Arc;

use async_graphql::{Context, Object};
use chrono::NaiveDate;
use uuid::Uuid;

use crate::model::session::Session;
use crate::model::Product;
use crate::repo::{self, AccountOutput, SearchElementAccount, TransactionOutput};
use crate::utils::{DatabasePool, RedisPool};
use crate::{identity_service::Identity, utils::ServiceResult};

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
    ) -> ServiceResult<Vec<SearchElementAccount>> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::get_accounts(database_pool.deref(), identity, search.as_deref())
            .await
            .map(|v| v.into_iter().map(|e| e.into()).collect())
    }

    async fn get_account(&self, ctx: &Context<'_>, id: Uuid) -> ServiceResult<AccountOutput> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::get_account(database_pool.deref(), identity, id).await
    }

    async fn get_account_by_access_token(
        &self,
        ctx: &Context<'_>,
        account_access_token: Session,
    ) -> ServiceResult<AccountOutput> {
        let redis_pool = ctx.data::<Arc<RedisPool>>()?;
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::get_account_by_access_token(
            database_pool.deref(),
            redis_pool.deref(),
            identity,
            account_access_token,
        )
        .await
    }

    #[graphql(entity)]
    async fn find_account_by_id(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> ServiceResult<AccountOutput> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::get_account(database_pool.deref(), identity, id).await
    }

    async fn get_transactions(
        &self,
        ctx: &Context<'_>,
        account_id: Uuid,
        transaction_filter_from: Option<String>,
        transaction_filter_to: Option<String>,
    ) -> ServiceResult<Vec<TransactionOutput>> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::get_transactions_by_account(
            database_pool.deref(),
            identity,
            account_id,
            transaction_filter_from
                .map(|s| {
                    NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                        .ok()
                        .map(|d| d.and_hms(0, 0, 0))
                })
                .flatten(),
            transaction_filter_to
                .map(|s| {
                    NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                        .ok()
                        .map(|d| d.and_hms(23, 59, 59))
                })
                .flatten(),
        )
        .await
    }

    async fn get_transaction(
        &self,
        ctx: &Context<'_>,
        account_id: Uuid,
        transaction_id: Uuid,
    ) -> ServiceResult<TransactionOutput> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::get_transaction_by_account(
            database_pool.deref(),
            identity,
            account_id,
            transaction_id,
        )
        .await
    }

    async fn get_own_transactions(
        &self,
        ctx: &Context<'_>,
        transaction_filter_from: Option<String>,
        transaction_filter_to: Option<String>,
    ) -> ServiceResult<Vec<TransactionOutput>> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::get_transactions_self(
            database_pool.deref(),
            identity,
            transaction_filter_from
                .map(|s| {
                    NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                        .ok()
                        .map(|d| d.and_hms(0, 0, 0))
                })
                .flatten(),
            transaction_filter_to
                .map(|s| {
                    NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                        .ok()
                        .map(|d| d.and_hms(23, 59, 59))
                })
                .flatten(),
        )
        .await
    }

    async fn get_own_transaction(
        &self,
        ctx: &Context<'_>,
        transaction_id: Uuid,
    ) -> ServiceResult<TransactionOutput> {
        let database_pool = ctx.data::<Arc<DatabasePool>>()?;
        let identity = ctx.data::<Identity>()?;
        repo::get_transaction_self(database_pool.deref(), identity, transaction_id).await
    }

    async fn get_products(&self, ctx: &Context<'_>) -> ServiceResult<Vec<Product>> {
        let identity = ctx.data::<Identity>()?;
        repo::get_products(identity)
    }

    async fn get_product(&self, ctx: &Context<'_>, id: String) -> ServiceResult<Product> {
        let identity = ctx.data::<Identity>()?;
        repo::get_product(identity, &id)
    }

    #[graphql(entity)]
    async fn find_product_by_id(&self, ctx: &Context<'_>, id: String) -> ServiceResult<Product> {
        let identity = ctx.data::<Identity>()?;
        repo::get_product(identity, &id)
    }
}
