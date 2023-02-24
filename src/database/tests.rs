use std::{collections::HashMap, ops::Add};

use chrono::{Duration, Utc};
use sqlx::PgPool;

use crate::{
    error::ServiceError,
    models::{
        Account, AuthMethod, AuthMethodType, AuthNfc, AuthPassword, CardType, CoinAmount, CoinType,
        Image, Payment, PaymentItem, Product, Role, TransactionItem,
    },
};

use super::{AppState, DatabaseConnection};

#[sqlx::test]
async fn test_session_crud(pool: PgPool) {
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

    let token = db
        .create_session_token(
            acc1.id,
            AuthMethodType::PasswordBased,
            Utc::now().add(Duration::minutes(30)),
            false,
        )
        .await
        .unwrap();
    let session = db
        .get_session_by_session_token(token.clone())
        .await
        .unwrap();
    let session = session.expect("there is a session for the token");

    assert_eq!(session.account, acc1);
    assert_eq!(session.auth_method, AuthMethodType::PasswordBased);
    assert_eq!(session.is_single_use, false);
    assert_eq!(session.token, token.clone());

    assert_eq!(
        db.get_sessions_by_account(acc1.id).await.unwrap(),
        vec![session.clone()]
    );

    db.delete_session_token(token.clone()).await.unwrap();
    assert_eq!(db.get_session_by_session_token(token).await.unwrap(), None);
    assert_eq!(db.get_sessions_by_account(acc1.id).await.unwrap(), vec![]);

    let token = db
        .create_session_token(
            acc1.id,
            AuthMethodType::PasswordBased,
            Utc::now().add(Duration::minutes(30)),
            false,
        )
        .await
        .unwrap();
    db.delete_account(acc1.id).await.unwrap();
    assert_eq!(db.get_session_by_session_token(token).await.unwrap(), None);
}

#[sqlx::test]
async fn test_account_crud(pool: PgPool) {
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
    acc1_clone.balance = CoinAmount(HashMap::new());
    acc1_clone.id = acc1.id;
    assert_eq!(acc1, acc1_clone);

    let acc2 = Account {
        name: "Rich Don".to_string(),
        email: "rich,don@example.com".to_string(),
        id: 0,
        balance: CoinAmount(HashMap::new()),
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
        db.get_account_by_auth_method(john_pw.to_request(acc1.id))
            .await
            .unwrap(),
        Some(acc1.clone())
    );
    assert_eq!(
        db.get_account_by_auth_method(john_nfc.to_request(acc1.id))
            .await
            .unwrap(),
        Some(acc1.clone())
    );
}

#[sqlx::test]
pub fn test_product_crud(pool: PgPool) {
    let product1 = Product {
        id: 0,
        price: CoinAmount([(CoinType::Cent, 150)].into_iter().collect()),
        bonus: CoinAmount(HashMap::new()),
        barcode: Some("barcode".to_string()),
        category: "category".to_string(),
        name: "Product 1".to_string(),
        nickname: Some("nick's test".to_string()),
        image: None,
        tags: vec![],
    };

    let product2 = Product {
        id: 0,
        price: CoinAmount(
            [(CoinType::Cent, 150), (CoinType::BottleStamp, 10)]
                .into_iter()
                .collect(),
        ),
        bonus: CoinAmount([(CoinType::BottleStamp, 1)].into_iter().collect()),
        barcode: Some("123891".to_string()),
        category: "kaltgetränk".to_string(),
        nickname: None,
        name: "testMate".to_string(),
        image: None,
        tags: vec!["koffein".to_string()],
    };

    let product3 = Product {
        id: 0,
        name: "Kaffee Crema".to_string(),
        price: CoinAmount(
            [(CoinType::Cent, 110), (CoinType::CoffeeStamp, 7)]
                .into_iter()
                .collect(),
        ),
        bonus: CoinAmount([(CoinType::CoffeeStamp, 1)].into_iter().collect()),
        barcode: None,
        category: "heißgetränk".to_string(),
        nickname: None,
        image: None,
        tags: vec![],
    };

    let app_state = AppState::from_pool(pool).await;
    let mut db = DatabaseConnection {
        connection: app_state.pool.acquire().await.unwrap(),
    };

    let mut product1_clone = product1.clone();
    let product1 = db.store_product(product1).await.unwrap();
    product1_clone.id = product1.id;
    assert_eq!(product1_clone, product1);
    assert!(product1.id != 0);

    let product2 = db.store_product(product2).await.unwrap();
    let product3 = db.store_product(product3).await.unwrap();

    let mut product1_no_image = product1.clone();
    product1_no_image.image = None;

    let mut products = db.get_all_products().await.unwrap();
    products.sort_by_key(|p| p.id);
    assert_eq!(
        products.as_slice(),
        &[
            product1_no_image.clone(),
            product2.clone(),
            product3.clone(),
        ]
    );

    let image2 = Image {
        data: vec![2, 2, 2, 2, 2, 2, 2, 2],
        mimetype: "image/jpeg".to_string(),
    };
    db.store_product_image(product2.id, image2.clone())
        .await
        .unwrap();
    assert_eq!(
        db.get_product_image(product1.id).await.unwrap(),
        product1.image
    );
    assert_eq!(
        db.get_product_image(product2.id).await.unwrap(),
        Some(image2.clone())
    );
    assert_eq!(db.get_product_image(product3.id).await.unwrap(), None);

    // images are not fetched by default
    assert_eq!(
        db.get_product_by_id(product2.id).await.unwrap(),
        Some(product2.clone())
    );

    db.delete_product_image(product2.id).await.unwrap();
    assert_eq!(
        db.get_product_image(product1.id).await.unwrap(),
        product1.image
    );
    assert_eq!(db.get_product_image(product2.id).await.unwrap(), None);

    db.delete_product(product1.id).await.unwrap();
    assert_eq!(
        db.store_product_image(product1.id, image2.clone()).await,
        Err(ServiceError::NotFound)
    );
    assert_eq!(
        db.delete_product(product1.id).await,
        Err(ServiceError::NotFound)
    );
    assert_eq!(db.get_product_image(product1.id).await, Ok(None));

    let mut products = db.get_all_products().await.unwrap();
    products.sort_by_key(|p| p.id);
    assert_eq!(products.as_slice(), &[product2.clone(), product3.clone(),]);
    assert_eq!(db.get_product_by_id(product1.id).await.unwrap(), None);
}

#[sqlx::test]
pub fn test_transaction(pool: PgPool) {
    let app_state = AppState::from_pool(pool).await;
    let mut db = DatabaseConnection {
        connection: app_state.pool.acquire().await.unwrap(),
    };

    // create some test data
    let acc1 = db
        .store_account(Account {
            name: "John Doe".to_string(),
            email: "john.doe@example.org".to_string(),
            id: 0,
            balance: CoinAmount([(CoinType::Cent, 368)].into_iter().collect()),
            role: Role::Basic,
            auth_methods: vec![],
        })
        .await
        .unwrap();

    let acc2 = db
        .store_account(Account {
            name: "Best Buyer".to_string(),
            email: "best@example.com".to_string(),
            id: 0,
            balance: CoinAmount(
                [(CoinType::Cent, 800), (CoinType::BottleStamp, 20)]
                    .into_iter()
                    .collect(),
            ),
            role: Role::Admin,
            auth_methods: vec![],
        })
        .await
        .unwrap();

    let product1 = db
        .store_product(Product {
            id: 0,
            price: CoinAmount([(CoinType::Cent, 150)].into_iter().collect()),
            bonus: CoinAmount(HashMap::new()),
            barcode: Some("barcode".to_string()),
            category: "category".to_string(),
            name: "Product 1".to_string(),
            nickname: Some("nick's test".to_string()),
            image: Some(Image {
                data: vec![0x1, 0x2, 0x3],
                mimetype: "image/png".to_string(),
            }),
            tags: vec![],
        })
        .await
        .unwrap();

    let product2 = db
        .store_product(Product {
            id: 0,
            price: CoinAmount(
                [(CoinType::Cent, 150), (CoinType::BottleStamp, 10)]
                    .into_iter()
                    .collect(),
            ),
            bonus: CoinAmount([(CoinType::BottleStamp, 1)].into_iter().collect()),
            barcode: Some("123891".to_string()),
            category: "kaltgetränk".to_string(),
            nickname: None,
            name: "testMate".to_string(),
            image: None,
            tags: vec!["koffein".to_string()],
        })
        .await
        .unwrap();

    let item1 = PaymentItem {
        effective_price: CoinAmount(
            [(CoinType::Cent, 150), (CoinType::CoffeeStamp, -1)]
                .into_iter()
                .collect(),
        ),
        product_id: Some(product1.id),
    };
    let item_no_product_id = PaymentItem {
        effective_price: CoinAmount([(CoinType::Cent, 112)].into_iter().collect()),
        product_id: None,
    };
    let item2 = PaymentItem {
        effective_price: CoinAmount([(CoinType::BottleStamp, 10)].into_iter().collect()),
        product_id: Some(product2.id),
    };
    let payment1 = Payment {
        account: acc1.id,
        items: vec![item1.clone(), item_no_product_id.clone()],
    };
    let tx1 = db.payment(payment1.clone()).await.unwrap();

    let mut product1_without_image = product1.clone();
    product1_without_image.image = None;

    assert!(tx1.id != 0);
    assert_eq!(tx1.account, acc1.id);
    assert_eq!(
        tx1.items.as_slice(),
        &[
            TransactionItem {
                effective_price: item1.effective_price.clone(),
                product: Some(product1_without_image),
            },
            TransactionItem {
                effective_price: item_no_product_id.effective_price.clone(),
                product: None,
            }
        ]
    );

    let tx2 = db
        .payment(Payment {
            account: acc2.id,
            items: vec![item_no_product_id],
        })
        .await
        .unwrap();

    // check that transactions are stored in db
    let r = db.get_transactions_by_account(tx1.account).await.unwrap();
    assert_eq!(r, vec![tx1.clone()]);

    let r = db.get_transactions_by_account(tx2.account).await.unwrap();
    assert_eq!(r, vec![tx2.clone()]);

    assert_eq!(
        db.get_transaction_by_id(tx1.id).await.unwrap(),
        Some(tx1.clone())
    );
    assert_eq!(db.get_transaction_by_id(42).await.unwrap(), None);

    // check balances are updated
    assert_eq!(
        db.get_account_by_id(acc1.id)
            .await
            .unwrap()
            .expect("have account")
            .balance,
        CoinAmount(
            [(CoinType::Cent, 106), (CoinType::CoffeeStamp, 1)]
                .into_iter()
                .collect()
        )
    );
    assert_eq!(
        db.get_account_by_id(acc2.id)
            .await
            .unwrap()
            .expect("have account")
            .balance,
        CoinAmount(
            [(CoinType::Cent, 800 - 112), (CoinType::BottleStamp, 20)]
                .into_iter()
                .collect(),
        )
    );

    // it should be possible to do the same payment again
    let tx3 = db.payment(payment1.clone()).await.unwrap();
    assert_eq!(tx3.items, tx1.items);
    assert_eq!(tx3.account, tx1.account);

    assert_eq!(
        db.get_transactions_by_account(tx3.account).await.unwrap(),
        vec![tx1.clone(), tx3.clone(),]
    );

    // if we delete the account, the id in the transaction is set to 0
    db.delete_account(tx3.account).await.unwrap();

    let mut tx3_anon = tx3.clone();
    tx3_anon.account = 0;
    let mut tx1_anon = tx1.clone();
    tx1_anon.account = 0;
    assert_eq!(
        db.get_transactions_by_account(0).await.unwrap(),
        vec![tx1_anon, tx3_anon]
    )
}
