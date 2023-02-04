#![allow(unused_variables)]

use std::time::Instant;

use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

use crate::error::ServiceResult;
use crate::models;

mod migration;

#[derive(Clone)]
pub struct Database {
    pub pool: Pool<Postgres>,
}

impl Database {
    pub async fn connect(url: &str) -> Database {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .expect("connect to database");

        let migrator = Migrator::new(migration::postgresql_migrations())
            .await
            .expect("load migrations");
        migrator.run(&pool).await.expect("run migrations");

        Database { pool }
    }

    pub async fn get_all_accounts(&self) -> ServiceResult<Vec<models::Account>> {
        panic!("TODO")
    }

    pub async fn get_account_by_id(&self, id: u64) -> ServiceResult<Option<models::Account>> {
        panic!("TODO")
    }

    pub async fn get_account_by_auth_method(
        &self,
        auth_method: Vec<u8>,
    ) -> ServiceResult<Option<models::Account>> {
        panic!("TODO")
    }

    pub async fn create_session_token(
        &self,
        account: u64,
        auth_method: models::AuthMethodType,
        valid_until: Instant,
        is_single_use: bool,
    ) -> ServiceResult<String> {
        panic!("TODO")
    }

    pub async fn delete_session_token(&self, session_token: String) -> ServiceResult<()> {
        panic!("TODO")
    }

    pub async fn get_session_by_session_token(
        &self,
        session_token: String,
    ) -> ServiceResult<Option<models::Session>> {
        panic!("TODO")
    }

    pub async fn store_account(&self, account: models::Account) -> ServiceResult<models::Account> {
        panic!("TODO")
    }

    pub async fn delete_account(&self, id: u64) -> ServiceResult<()> {
        panic!("TODO")
    }

    pub async fn get_all_products(&self) -> ServiceResult<Vec<models::Product>> {
        panic!("TODO")
    }

    pub async fn get_product_by_id(&self, id: u64) -> ServiceResult<Option<models::Product>> {
        panic!("TODO")
    }

    pub async fn store_product(&self, product: models::Product) -> ServiceResult<models::Product> {
        panic!("TODO")
    }

    pub async fn delete_product(&self, id: u64) -> ServiceResult<()> {
        panic!("TODO")
    }

    pub async fn get_product_image(&self, id: u64) -> ServiceResult<Option<models::Image>> {
        panic!("TODO")
    }

    pub async fn store_product_image(&self, id: u64, image: models::Image) -> ServiceResult<()> {
        panic!("TODO")
    }

    pub async fn delete_product_image(&self, id: u64) -> ServiceResult<()> {
        panic!("TODO")
    }

    pub async fn get_transactions_by_account(
        &self,
        account_id: u64,
    ) -> ServiceResult<Vec<models::Transaction>> {
        panic!("TODO")
    }

    pub async fn get_transaction_by_id(
        &self,
        id: u64,
    ) -> ServiceResult<Option<models::Transaction>> {
        panic!("TODO")
    }

    pub async fn payment(&self, payment: models::Payment) -> ServiceResult<models::Transaction> {
        panic!("TODO")
    }
}
