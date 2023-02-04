use sqlx::{Pool, Postgres};

use crate::error::ServiceResult;
use crate::models;

#[derive(Clone)]
pub struct Database {
    pub pool: Pool<Postgres>,
}

impl Database {
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
