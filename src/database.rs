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
            .connect(&url)
            .await
            .expect("connect to database");

        let migrator = Migrator::new(migration::postgresql_migrations()).await.expect("load migrations");
        migrator.run(&pool).await.expect("run migrations");

        Database { pool }
    }

    pub async fn get_all_accounts(&self) -> ServiceResult<Vec<models::Account>> {
        panic!("TODO")
    }

    pub async fn get_account_by_id(&self, id: u64) -> ServiceResult<Option<models::Account>> {
        panic!("TODO")
    }

    pub async fn store_account(&self, account: models::Account) -> ServiceResult<models::Account> {
        panic!("TODO")
    }

    pub async fn delete_account(&self, id: u64) -> ServiceResult<()> {
        panic!("TODO")
    }
}
