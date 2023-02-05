use std::collections::HashMap;

use futures::StreamExt;
use sqlx::PgPool;

use crate::{error::ServiceResult, models::{Account, Role, AuthMethod, AuthPassword, CoinAmount}};

use super::{AppState, DatabaseConnection};

#[sqlx::test]
async fn test_account_store_and_get(pool: PgPool) {
    env_logger::init();
    let app_state = AppState::from_pool(pool).await;
    let mut db = DatabaseConnection { connection: app_state.pool.acquire().await.unwrap() };

    let john_pw = AuthPassword { username: "johndoe".to_string(), password_hash: vec![13u8; 32] };
    let acc1 = Account {
        name: "John Doe".to_string(),
        email: "john.doe@example.org".to_string(),
        id: 0,
        balance: CoinAmount(HashMap::new()),
        role: Role::Basic,
        auth_methods: vec![
            AuthMethod::PasswordBased(john_pw),
        ]
    };
    let mut acc1_clone = acc1.clone();
    let acc1 = db.store_account(acc1).await.unwrap();
    assert!(acc1.id != 0);
    acc1_clone.balance = CoinAmount::zero();
    acc1_clone.id = acc1.id;
    assert_eq!(acc1, acc1_clone);

    let acc2 = Account {
        name: "Rich Don".to_string(),
        email: "rich,don@example.com".to_string(),
        id: 0,
        balance: CoinAmount::zero(),
        role: Role::Member,
        auth_methods: vec![]
    };
    let acc2 = db.store_account(acc2).await.unwrap();

    let mut all_accounts = db.get_all_accounts().await.unwrap();
    all_accounts.sort_by_key(|acc| acc.id);
    assert_eq!(all_accounts, vec![acc1, acc2]);

}

#[sqlx::test]
async fn test_execute_many(pool: PgPool) {
    let q = sqlx::query(r#"SELECT 1; SELECT 2"#);
    let mut r = q.execute_many(&pool).await;
    while let Some(row) = r.next().await {
        let row = row.unwrap();
        dbg!(row);
    }
}