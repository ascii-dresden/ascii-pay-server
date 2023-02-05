use std::{collections::HashMap, ops::Add};

use chrono::{Utc, Duration};
use futures::StreamExt;
use sqlx::PgPool;

use crate::models::{Account, AuthMethod, AuthNfc, AuthPassword, CardType, CoinAmount, Role, AuthMethodType};

use super::{AppState, DatabaseConnection};

#[sqlx::test]
async fn test_session_crud(pool: PgPool) {
    env_logger::init();
    let app_state = AppState::from_pool(pool).await;
    let mut db = DatabaseConnection {
        connection: app_state.pool.acquire().await.unwrap(),
    };

    let john_pw = AuthMethod::PasswordBased(AuthPassword {
        username: "johndoe".to_string(),
        password_hash: vec![13u8; 32],
    });
    let acc1 = Account {
        name: "John Doe".to_string(),
        email: "john.doe@example.org".to_string(),
        id: 0,
        balance: CoinAmount(HashMap::new()),
        role: Role::Basic,
        auth_methods: vec![john_pw.clone()],
    };
    let acc1 = db.store_account(acc1).await.unwrap();

    let token = db.create_session_token(acc1.id, AuthMethodType::PasswordBased, Utc::now().add(Duration::minutes(30)), false).await.unwrap();
    let session = db.get_session_by_session_token(token.clone()).await.unwrap();
    let session = session.expect("there is a session for the token");

    assert_eq!(session.account, acc1);
    assert_eq!(session.auth_method, AuthMethodType::PasswordBased);
    assert_eq!(session.is_single_use, false);
    assert_eq!(session.token, token.clone());

    db.delete_session_token(token.clone()).await.unwrap();
    assert_eq!(db.get_session_by_session_token(token).await.unwrap(), None);

    let token = db.create_session_token(acc1.id, AuthMethodType::PasswordBased, Utc::now().add(Duration::minutes(30)), false).await.unwrap();
    db.delete_account(acc1.id).await.unwrap();
    assert_eq!(db.get_session_by_session_token(token).await.unwrap(), None);

}

#[sqlx::test]
async fn test_account_crud(pool: PgPool) {
    env_logger::init();
    let app_state = AppState::from_pool(pool).await;
    let mut db = DatabaseConnection {
        connection: app_state.pool.acquire().await.unwrap(),
    };

    let john_pw = AuthMethod::PasswordBased(AuthPassword {
        username: "johndoe".to_string(),
        password_hash: vec![13u8; 32],
    });
    let acc1 = Account {
        name: "John Doe".to_string(),
        email: "john.doe@example.org".to_string(),
        id: 0,
        balance: CoinAmount(HashMap::new()),
        role: Role::Basic,
        auth_methods: vec![john_pw.clone()],
    };
    let mut acc1_clone = acc1.clone();
    let mut acc1 = db.store_account(acc1).await.unwrap();
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
        auth_methods: vec![],
    };
    let acc2 = db.store_account(acc2).await.unwrap();

    let mut all_accounts = db.get_all_accounts().await.unwrap();
    all_accounts.sort_by_key(|acc| acc.id);
    assert_eq!(all_accounts, vec![acc1.clone(), acc2.clone()]);

    let john_nfc = AuthMethod::NfcBased(AuthNfc {
        card_id: vec![1; 32],
        data: vec![],
        card_type: CardType::NfcId,
        name: "My NFC Card".to_string(),
    });
    acc1.auth_methods.push(john_nfc.clone());
    let acc1_clone = acc1.clone();
    let acc1 = db.store_account(acc1).await.unwrap();
    assert_eq!(acc1_clone, acc1);
    assert_eq!(
        db.get_account_by_id(acc1.id).await.unwrap(),
        Some(acc1.clone())
    );
    assert_eq!(db.get_all_accounts().await.unwrap().len(), 2);

    assert_eq!(
        db.get_account_by_id(acc1.id).await.unwrap(),
        Some(acc1.clone())
    );
    assert_eq!(
        db.get_account_by_id(acc2.id).await.unwrap(),
        Some(acc2.clone())
    );
    assert_eq!(db.get_account_by_id(0).await.unwrap(), None);
    assert_eq!(db.get_account_by_id(123213).await.unwrap(), None);

    assert_eq!(
        db.get_account_by_auth_method(john_pw.to_request(acc1.id).login_key())
            .await
            .unwrap(),
        Some(acc1.clone())
    );
    assert_eq!(
        db.get_account_by_auth_method(john_nfc.to_request(acc1.id).login_key())
            .await
            .unwrap(),
        Some(acc1.clone())
    );
}
