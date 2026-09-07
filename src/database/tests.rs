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
        enable_monthly_mail_report: false,
        enable_automatic_stamp_usage: true,
        status: None,
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
    assert!(!session.is_single_use);
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
        enable_monthly_mail_report: false,
        enable_automatic_stamp_usage: true,
        status: None,
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
        enable_monthly_mail_report: false,
        enable_automatic_stamp_usage: true,
        status: None,
    };
    let acc2 = db.store_account(acc2).await.unwrap();

    let mut all_accounts = db.get_all_accounts().await.unwrap();
    all_accounts.sort_by_key(|acc| acc.id);
    assert_eq!(all_accounts, vec![acc1.clone(), acc2.clone()]);

    let john_nfc = AuthMethod::NfcBased(AuthNfc {
        card_id: vec![1; 32],
        data: vec![],
        card_type: CardType::GenericNfc,
        name: "My NFC Card".to_string(),
        depends_on_session: None,
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
        purchase_tax: 19,
        image: None,
        print_lists: vec![],
        tags: vec![],
        status_prices: Vec::new(),
        stocked: false,
        target_quantity: 0,
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
        purchase_tax: 19,
        image: None,
        print_lists: vec![],
        tags: vec!["koffein".to_string()],
        status_prices: Vec::new(),
        stocked: false,
        target_quantity: 0,
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
        purchase_tax: 19,
        image: None,
        print_lists: vec![],
        tags: vec![],
        status_prices: Vec::new(),
        stocked: false,
        target_quantity: 0,
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
            enable_monthly_mail_report: false,
            enable_automatic_stamp_usage: true,
            status: None,
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
            enable_monthly_mail_report: false,
            enable_automatic_stamp_usage: true,
            status: None,
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
            purchase_tax: 19,
            image: Some(Image {
                data: vec![0x1, 0x2, 0x3],
                mimetype: "image/png".to_string(),
            }),
            print_lists: vec![],
            tags: vec![],
            status_prices: Vec::new(),
            stocked: false,
            target_quantity: 0,
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
            purchase_tax: 19,
            image: None,
            print_lists: vec![],
            tags: vec!["koffein".to_string()],
            status_prices: Vec::new(),
            stocked: false,
            target_quantity: 0,
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
        authorization: None,
    };
    let tx1 = db
        .payment(payment1.clone(), Utc::now(), false)
        .await
        .unwrap();

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
        .payment(
            Payment {
                account: acc2.id,
                items: vec![item_no_product_id],
                authorization: None,
            },
            Utc::now(),
            false,
        )
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
    let tx3 = db
        .payment(payment1.clone(), Utc::now(), false)
        .await
        .unwrap();
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

// ---------------------------------------------------------------------------
// Inventory & purchase rework
// ---------------------------------------------------------------------------

use crate::models::{
    InventoryCheckCompletion, InventoryCheckState, Purchase, PurchaseItem, PurchaseState,
};
use chrono::NaiveDate;

fn test_account(name: &str, role: Role) -> Account {
    Account {
        name: name.to_string(),
        email: format!("{}@example.org", name.to_lowercase().replace(' ', ".")),
        id: 0,
        balance: CoinAmount(HashMap::new()),
        role,
        auth_methods: vec![],
        enable_monthly_mail_report: false,
        enable_automatic_stamp_usage: true,
        status: None,
    }
}

fn test_product(
    name: &str,
    category: &str,
    barcode: Option<&str>,
    stocked: bool,
    target: i32,
) -> Product {
    Product {
        id: 0,
        name: name.to_string(),
        price: CoinAmount([(CoinType::Cent, 150)].into_iter().collect()),
        bonus: CoinAmount(HashMap::new()),
        purchase_tax: 19,
        nickname: None,
        image: None,
        barcode: barcode.map(str::to_string),
        category: category.to_string(),
        print_lists: vec![],
        tags: vec![],
        status_prices: Vec::new(),
        stocked,
        target_quantity: target,
    }
}

fn test_purchase(
    state: PurchaseState,
    timestamp: chrono::DateTime<Utc>,
    items: Vec<PurchaseItem>,
) -> Purchase {
    Purchase {
        id: 0,
        name: "Test purchase".to_string(),
        store: "Store".to_string(),
        timestamp,
        state,
        purchased_by_account_id: None,
        finalized_at: None,
        finalized_by_account_id: None,
        items,
    }
}

fn test_item(
    name: &str,
    product: Option<&Product>,
    size: i32,
    count: i32,
    cents: i32,
    collected: bool,
) -> PurchaseItem {
    PurchaseItem {
        id: 0,
        name: name.to_string(),
        container_size: size,
        container_count: count,
        container_cents: cents,
        best_before: None,
        barcode: None,
        collected,
        product: product.cloned(),
    }
}

#[sqlx::test]
async fn test_purchase_lifecycle(pool: PgPool) {
    let app_state = AppState::from_pool(pool).await;
    let mut db = DatabaseConnection {
        connection: app_state.pool.acquire().await.unwrap(),
    };

    let purchaser = db
        .store_account(test_account("Purchaser", Role::Purchaser))
        .await
        .unwrap();
    let product = db
        .store_product(test_product("Mate", "Drinks", Some("4711"), true, 10))
        .await
        .unwrap();

    // create a draft without items
    let purchase = db
        .create_purchase(test_purchase(PurchaseState::Draft, Utc::now(), vec![]))
        .await
        .unwrap();
    assert!(purchase.id != 0);
    assert_eq!(purchase.state, PurchaseState::Draft);
    assert_eq!(purchase.name, "Test purchase");
    assert_eq!(purchase.finalized_at, None);
    assert_eq!(purchase.finalized_by_account_id, None);
    assert!(purchase.items.is_empty());

    // add an item
    let mut item = test_item("Mate crate", Some(&product), 20, 1, 1999, true);
    item.best_before = Some(NaiveDate::from_ymd_opt(2027, 1, 31).unwrap());
    item.barcode = Some("4711".to_string());
    let item1 = db.add_purchase_item(purchase.id, item).await.unwrap();
    assert!(item1.id != 0);
    assert_eq!(item1.name, "Mate crate");
    assert_eq!(
        item1.best_before,
        Some(NaiveDate::from_ymd_opt(2027, 1, 31).unwrap())
    );
    assert_eq!(item1.barcode.as_deref(), Some("4711"));
    assert!(item1.collected);
    assert_eq!(item1.product, Some(product.clone()));

    // update the item
    let mut changed = item1.clone();
    changed.container_count = 2;
    changed.best_before = None;
    changed.product = None;
    let item1 = db
        .update_purchase_item(purchase.id, item1.id, changed)
        .await
        .unwrap();
    assert_eq!(item1.container_count, 2);
    assert_eq!(item1.best_before, None);
    assert_eq!(item1.product, None);

    // unknown item / purchase
    assert_eq!(
        db.update_purchase_item(purchase.id, 424242, item1.clone())
            .await,
        Err(ServiceError::NotFound)
    );
    assert_eq!(
        db.add_purchase_item(424242, item1.clone()).await,
        Err(ServiceError::NotFound)
    );
    assert_eq!(
        db.delete_purchase_item(purchase.id, 424242).await,
        Err(ServiceError::NotFound)
    );

    // a second, uncollected item
    let item2 = db
        .add_purchase_item(
            purchase.id,
            test_item("Planned", Some(&product), 6, 1, 0, false),
        )
        .await
        .unwrap();
    assert!(!item2.collected);

    let loaded = db.get_purchase_by_id(purchase.id).await.unwrap().unwrap();
    assert_eq!(loaded.items, vec![item1.clone(), item2.clone()]);

    // finalize drops uncollected items
    let finalized = db
        .finalize_purchase(purchase.id, Some(purchaser.id))
        .await
        .unwrap();
    assert_eq!(finalized.state, PurchaseState::Finalized);
    assert!(finalized.finalized_at.is_some());
    assert_eq!(finalized.finalized_by_account_id, Some(purchaser.id));
    assert_eq!(finalized.items, vec![item1.clone()]);

    // every write is rejected now
    assert!(matches!(
        db.add_purchase_item(purchase.id, item2.clone()).await,
        Err(ServiceError::Conflict(_))
    ));
    assert!(matches!(
        db.update_purchase_item(purchase.id, item1.id, item1.clone())
            .await,
        Err(ServiceError::Conflict(_))
    ));
    assert!(matches!(
        db.delete_purchase_item(purchase.id, item1.id).await,
        Err(ServiceError::Conflict(_))
    ));
    assert!(matches!(
        db.update_purchase(finalized.clone(), false).await,
        Err(ServiceError::Conflict(_))
    ));
    assert!(matches!(
        db.finalize_purchase(purchase.id, None).await,
        Err(ServiceError::Conflict(_))
    ));
    assert_eq!(
        db.get_purchase_by_id(purchase.id)
            .await
            .unwrap()
            .unwrap()
            .items,
        vec![item1.clone()]
    );

    assert_eq!(
        db.get_purchases(Some(PurchaseState::Finalized))
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        db.get_purchases(Some(PurchaseState::Draft))
            .await
            .unwrap()
            .len(),
        0
    );
    assert_eq!(db.get_purchases(None).await.unwrap().len(), 1);

    // reopen
    let reopened = db.reopen_purchase(purchase.id).await.unwrap();
    assert_eq!(reopened.state, PurchaseState::Draft);
    assert_eq!(reopened.finalized_at, None);
    assert_eq!(reopened.finalized_by_account_id, None);
    assert!(matches!(
        db.reopen_purchase(purchase.id).await,
        Err(ServiceError::Conflict(_))
    ));
    assert_eq!(
        db.reopen_purchase(424242).await,
        Err(ServiceError::NotFound)
    );

    // writes work again
    let item3 = db
        .add_purchase_item(purchase.id, test_item("Again", None, 1, 1, 100, true))
        .await
        .unwrap();
    let loaded = db.get_purchase_by_id(purchase.id).await.unwrap().unwrap();
    assert_eq!(loaded.items, vec![item1.clone(), item3.clone()]);

    // header-only update keeps the items
    let mut header = loaded.clone();
    header.name = "Renamed".to_string();
    header.items = vec![];
    let updated = db.update_purchase(header, false).await.unwrap();
    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.items, vec![item1.clone(), item3.clone()]);

    // full update replaces the items
    let mut replaced = updated.clone();
    replaced.items = vec![test_item("Only", Some(&product), 2, 3, 400, true)];
    let updated = db.update_purchase(replaced, true).await.unwrap();
    assert_eq!(updated.items.len(), 1);
    assert_eq!(updated.items[0].name, "Only");

    db.delete_purchase_item(purchase.id, updated.items[0].id)
        .await
        .unwrap();
    assert!(db
        .get_purchase_by_id(purchase.id)
        .await
        .unwrap()
        .unwrap()
        .items
        .is_empty());

    // creating a finalized purchase directly records the finalizer
    let mut legacy = test_purchase(
        PurchaseState::Finalized,
        Utc::now(),
        vec![test_item("Legacy", Some(&product), 1, 1, 1, true)],
    );
    legacy.finalized_by_account_id = Some(purchaser.id);
    let legacy = db.create_purchase(legacy).await.unwrap();
    assert_eq!(legacy.state, PurchaseState::Finalized);
    assert!(legacy.finalized_at.is_some());
    assert_eq!(legacy.finalized_by_account_id, Some(purchaser.id));
    assert_eq!(legacy.items.len(), 1);

    let by_product = db
        .get_purchases_by_product_id(product.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        by_product.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![legacy.id]
    );

    db.delete_purchase(purchase.id).await.unwrap();
    assert_eq!(
        db.delete_purchase(purchase.id).await,
        Err(ServiceError::NotFound)
    );
}

#[sqlx::test]
async fn test_product_barcode_and_inventory_update(pool: PgPool) {
    let app_state = AppState::from_pool(pool).await;
    let mut db = DatabaseConnection {
        connection: app_state.pool.acquire().await.unwrap(),
    };

    let p1 = db
        .store_product(test_product("First", "A", Some("4711"), false, 0))
        .await
        .unwrap();
    let p2 = db
        .store_product(test_product("Second", "A", Some("4711"), false, 0))
        .await
        .unwrap();
    let p3 = db
        .store_product(test_product("Third", "A", None, true, 3))
        .await
        .unwrap();

    assert_eq!(p3.stocked, true);
    assert_eq!(p3.target_quantity, 3);
    assert_eq!(db.get_product_by_id(p3.id).await.unwrap(), Some(p3.clone()));

    // lowest id wins on duplicates
    assert_eq!(
        db.get_product_by_barcode("4711").await.unwrap(),
        Some(p1.clone())
    );
    assert_eq!(db.get_product_by_barcode("0815").await.unwrap(), None);
    assert_eq!(db.get_product_by_barcode("").await.unwrap(), None);

    // bulk update
    db.update_products_inventory(&[(p1.id, true, 12), (p2.id, true, 4)])
        .await
        .unwrap();
    let p1 = db.get_product_by_id(p1.id).await.unwrap().unwrap();
    let p2 = db.get_product_by_id(p2.id).await.unwrap().unwrap();
    assert_eq!((p1.stocked, p1.target_quantity), (true, 12));
    assert_eq!((p2.stocked, p2.target_quantity), (true, 4));

    // unknown id: nothing applied
    assert_eq!(
        db.update_products_inventory(&[(p1.id, false, 0), (424242, true, 1)])
            .await,
        Err(ServiceError::NotFound)
    );
    let p1 = db.get_product_by_id(p1.id).await.unwrap().unwrap();
    assert_eq!((p1.stocked, p1.target_quantity), (true, 12));

    // stocked products show up in the inventory, ordered by category, name
    let inventory = db.get_inventory().await.unwrap();
    assert_eq!(
        inventory.iter().map(|r| r.product.id).collect::<Vec<_>>(),
        vec![p1.id, p2.id, p3.id]
    );
    assert_eq!(inventory[0].target_quantity, 12);
    assert_eq!(inventory[0].last_counted_quantity, None);
    assert_eq!(inventory[0].estimated_quantity, None);
    assert_eq!(inventory[0].last_purchase, None);
}

#[sqlx::test]
async fn test_inventory_check_lifecycle(pool: PgPool) {
    let app_state = AppState::from_pool(pool).await;
    let mut db = DatabaseConnection {
        connection: app_state.pool.acquire().await.unwrap(),
    };

    let admin = db
        .store_account(test_account("Admin", Role::Admin))
        .await
        .unwrap();
    let mate = db
        .store_product(test_product("Mate", "Drinks", None, true, 10))
        .await
        .unwrap();
    let cola = db
        .store_product(test_product("Cola", "Drinks", None, true, 5))
        .await
        .unwrap();
    let chips = db
        .store_product(test_product("Chips", "Snacks", None, true, 8))
        .await
        .unwrap();
    let _unstocked = db
        .store_product(test_product("Old", "Snacks", None, false, 0))
        .await
        .unwrap();

    // purchase history: Mate comes in containers of 6 for 899 cents (a draft one must not count)
    db.create_purchase(test_purchase(
        PurchaseState::Finalized,
        Utc::now() - Duration::days(2),
        vec![test_item("Mate", Some(&mate), 6, 2, 899, true)],
    ))
    .await
    .unwrap();
    db.create_purchase(test_purchase(
        PurchaseState::Draft,
        Utc::now() - Duration::days(1),
        vec![test_item("Mate", Some(&mate), 24, 1, 2999, true)],
    ))
    .await
    .unwrap();

    let check = db
        .create_inventory_check("first", Some(admin.id))
        .await
        .unwrap();
    assert!(check.id != 0);
    assert_eq!(check.state, InventoryCheckState::Draft);
    assert_eq!(check.note, "first");
    assert_eq!(check.started_by_account_id, Some(admin.id));
    assert_eq!(check.completed_at, None);
    assert_eq!(check.generated_purchase_id, None);
    assert_eq!(check.item_count, 3);
    assert_eq!(check.counted_count, 0);
    // ordered by category, then name
    assert_eq!(
        check.items.iter().map(|i| i.product.id).collect::<Vec<_>>(),
        vec![cola.id, mate.id, chips.id]
    );
    assert_eq!(check.items[1].target_quantity, 10);
    assert!(check.items.iter().all(|i| i.counted_quantity.is_none()));

    // only one draft at a time
    assert!(matches!(
        db.create_inventory_check("second", None).await,
        Err(ServiceError::Conflict(_))
    ));
    assert_eq!(
        db.get_inventory_checks(Some(InventoryCheckState::Draft))
            .await
            .unwrap()
            .len(),
        1
    );

    // count
    let item = db
        .set_inventory_check_item_count(check.id, mate.id, Some(3))
        .await
        .unwrap();
    assert_eq!(item.product.id, mate.id);
    assert_eq!(item.target_quantity, 10);
    assert_eq!(item.counted_quantity, Some(3));
    db.set_inventory_check_item_count(check.id, cola.id, Some(5))
        .await
        .unwrap();
    assert_eq!(
        db.set_inventory_check_item_count(check.id, 424242, Some(1))
            .await,
        Err(ServiceError::NotFound)
    );
    assert_eq!(
        db.set_inventory_check_item_count(424242, mate.id, Some(1))
            .await,
        Err(ServiceError::NotFound)
    );
    let loaded = db
        .get_inventory_check_by_id(check.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.counted_count, 2);

    // complete with generation: Mate is 7 short -> ceil(7/6) = 2 containers at 899 cents;
    // Cola is not below target; Chips is uncounted and therefore skipped.
    let (completed, purchase) = db
        .complete_inventory_check(
            check.id,
            InventoryCheckCompletion {
                completed_by_account_id: Some(admin.id),
                generate_purchase: true,
                purchase_store: "Metro".to_string(),
                purchase_name: "Inventory 2026-09-07".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(completed.state, InventoryCheckState::Completed);
    assert!(completed.completed_at.is_some());
    assert_eq!(completed.completed_by_account_id, Some(admin.id));
    let purchase = purchase.expect("a purchase was generated");
    assert_eq!(completed.generated_purchase_id, Some(purchase.id));
    assert_eq!(purchase.state, PurchaseState::Draft);
    assert_eq!(purchase.name, "Inventory 2026-09-07");
    assert_eq!(purchase.store, "Metro");
    assert_eq!(purchase.purchased_by_account_id, Some(admin.id));
    assert_eq!(purchase.items.len(), 1);
    let generated = &purchase.items[0];
    assert_eq!(generated.name, "Mate");
    assert_eq!(generated.product.as_ref().map(|p| p.id), Some(mate.id));
    assert_eq!(generated.container_size, 6);
    assert_eq!(generated.container_count, 2);
    assert_eq!(generated.container_cents, 899);
    assert!(!generated.collected);

    // completed checks are read-only
    assert!(matches!(
        db.complete_inventory_check(
            check.id,
            InventoryCheckCompletion {
                completed_by_account_id: None,
                generate_purchase: false,
                purchase_store: String::new(),
                purchase_name: String::new(),
            }
        )
        .await,
        Err(ServiceError::Conflict(_))
    ));
    assert!(matches!(
        db.set_inventory_check_item_count(check.id, mate.id, Some(4))
            .await,
        Err(ServiceError::Conflict(_))
    ));

    // second check: nothing below target -> no purchase
    let check2 = db.create_inventory_check("", None).await.unwrap();
    db.set_inventory_check_item_count(check2.id, cola.id, Some(9))
        .await
        .unwrap();
    let (completed2, purchase2) = db
        .complete_inventory_check(
            check2.id,
            InventoryCheckCompletion {
                completed_by_account_id: None,
                generate_purchase: true,
                purchase_store: String::new(),
                purchase_name: "x".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(purchase2, None);
    assert_eq!(completed2.generated_purchase_id, None);
    assert_eq!(completed2.state, InventoryCheckState::Completed);

    // third check: product without purchase history -> container size 1, cents 0
    let check3 = db.create_inventory_check("", None).await.unwrap();
    db.set_inventory_check_item_count(check3.id, chips.id, Some(2))
        .await
        .unwrap();
    let (_, purchase3) = db
        .complete_inventory_check(
            check3.id,
            InventoryCheckCompletion {
                completed_by_account_id: None,
                generate_purchase: true,
                purchase_store: String::new(),
                purchase_name: "y".to_string(),
            },
        )
        .await
        .unwrap();
    let purchase3 = purchase3.unwrap();
    assert_eq!(purchase3.items.len(), 1);
    assert_eq!(purchase3.items[0].container_size, 1);
    assert_eq!(purchase3.items[0].container_count, 6);
    assert_eq!(purchase3.items[0].container_cents, 0);

    // fourth check without generation
    let check4 = db.create_inventory_check("", None).await.unwrap();
    let (completed4, purchase4) = db
        .complete_inventory_check(
            check4.id,
            InventoryCheckCompletion {
                completed_by_account_id: None,
                generate_purchase: false,
                purchase_store: String::new(),
                purchase_name: String::new(),
            },
        )
        .await
        .unwrap();
    assert_eq!(purchase4, None);
    assert_eq!(completed4.generated_purchase_id, None);

    // listing: newest first, summaries without items
    let all = db.get_inventory_checks(None).await.unwrap();
    assert_eq!(
        all.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![check4.id, check3.id, check2.id, check.id]
    );
    assert!(all.iter().all(|c| c.items.is_empty()));
    assert_eq!(all[3].item_count, 3);
    assert_eq!(all[3].counted_count, 2);
    assert_eq!(
        db.get_inventory_checks(Some(InventoryCheckState::Draft))
            .await
            .unwrap()
            .len(),
        0
    );

    // deleting the generated purchase detaches it from the check
    db.delete_purchase(purchase.id).await.unwrap();
    let loaded = db
        .get_inventory_check_by_id(check.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.generated_purchase_id, None);

    db.delete_inventory_check(check.id).await.unwrap();
    assert_eq!(
        db.delete_inventory_check(check.id).await,
        Err(ServiceError::NotFound)
    );
    assert_eq!(db.get_inventory_check_by_id(check.id).await.unwrap(), None);
}

#[sqlx::test]
async fn test_inventory_estimate(pool: PgPool) {
    let app_state = AppState::from_pool(pool).await;
    let mut db = DatabaseConnection {
        connection: app_state.pool.acquire().await.unwrap(),
    };

    let buyer = db
        .store_account(test_account("Buyer", Role::Member))
        .await
        .unwrap();
    let mate = db
        .store_product(test_product("Mate", "Drinks", None, true, 10))
        .await
        .unwrap();

    // a purchase before the count must not influence the estimate
    db.create_purchase(test_purchase(
        PurchaseState::Finalized,
        Utc::now() - Duration::days(3),
        vec![test_item("Mate", Some(&mate), 6, 5, 899, true)],
    ))
    .await
    .unwrap();

    // never counted -> no estimate, but last purchase is known
    let row = &db.get_inventory().await.unwrap()[0];
    assert_eq!(row.last_counted_quantity, None);
    assert_eq!(row.estimated_quantity, None);
    assert_eq!(
        row.last_purchase.as_ref().map(|lp| lp.container_size),
        Some(6)
    );

    let check = db.create_inventory_check("", None).await.unwrap();
    db.set_inventory_check_item_count(check.id, mate.id, Some(4))
        .await
        .unwrap();
    let (completed, _) = db
        .complete_inventory_check(
            check.id,
            InventoryCheckCompletion {
                completed_by_account_id: None,
                generate_purchase: false,
                purchase_store: String::new(),
                purchase_name: String::new(),
            },
        )
        .await
        .unwrap();

    let row = &db.get_inventory().await.unwrap()[0];
    assert_eq!(row.last_counted_quantity, Some(4));
    assert_eq!(row.last_counted_at, completed.completed_at);
    assert_eq!(row.estimated_quantity, Some(4));

    // a finalized purchase after the count adds 2 * 6 units; a draft one does not count
    let after = Utc::now() + Duration::seconds(5);
    let newer = db
        .create_purchase(test_purchase(
            PurchaseState::Finalized,
            after,
            vec![test_item("Mate", Some(&mate), 6, 2, 949, true)],
        ))
        .await
        .unwrap();
    db.create_purchase(test_purchase(
        PurchaseState::Draft,
        after,
        vec![test_item("Mate", Some(&mate), 6, 100, 949, true)],
    ))
    .await
    .unwrap();

    let row = &db.get_inventory().await.unwrap()[0];
    assert_eq!(row.estimated_quantity, Some(16));
    let last_purchase = row.last_purchase.as_ref().unwrap();
    assert_eq!(last_purchase.purchase_id, newer.id);
    assert_eq!(last_purchase.container_size, 6);
    assert_eq!(last_purchase.container_cents, 949);
    assert_eq!(last_purchase.timestamp, newer.timestamp);

    // a sale after the count removes one unit per transaction item
    db.payment(
        Payment {
            account: buyer.id,
            items: vec![
                PaymentItem {
                    effective_price: CoinAmount([(CoinType::Cent, 150)].into_iter().collect()),
                    product_id: Some(mate.id),
                },
                PaymentItem {
                    effective_price: CoinAmount([(CoinType::Cent, 150)].into_iter().collect()),
                    product_id: Some(mate.id),
                },
            ],
            authorization: None,
        },
        after,
        false,
    )
    .await
    .unwrap();

    let row = &db.get_inventory().await.unwrap()[0];
    assert_eq!(row.estimated_quantity, Some(14));

    // a newer count resets the baseline
    let check2 = db.create_inventory_check("", None).await.unwrap();
    db.set_inventory_check_item_count(check2.id, mate.id, Some(20))
        .await
        .unwrap();
    db.complete_inventory_check(
        check2.id,
        InventoryCheckCompletion {
            completed_by_account_id: None,
            generate_purchase: false,
            purchase_store: String::new(),
            purchase_name: String::new(),
        },
    )
    .await
    .unwrap();
    // the purchase and sale at `after` (5s in the future) are still newer than this count
    let row = &db.get_inventory().await.unwrap()[0];
    assert_eq!(row.last_counted_quantity, Some(20));
    assert_eq!(row.estimated_quantity, Some(20 + 12 - 2));
}

#[sqlx::test]
async fn test_shopping_list_lifecycle(pool: PgPool) {
    use crate::models::{ShoppingListItem, ShoppingListState};

    let app_state = AppState::from_pool(pool).await;
    let mut db = DatabaseConnection {
        connection: app_state.pool.acquire().await.unwrap(),
    };

    let member = db
        .store_account(test_account("List Member", Role::Member))
        .await
        .unwrap();
    let purchaser = db
        .store_account(test_account("List Purchaser", Role::Purchaser))
        .await
        .unwrap();
    let product = db
        .store_product(test_product("Mate", "Drinks", Some("4711"), true, 10))
        .await
        .unwrap();

    let new_item =
        |name: &str, product: Option<&Product>, created_by: Option<u64>| ShoppingListItem {
            id: 0,
            name: name.to_string(),
            quantity: 2,
            note: "note".to_string(),
            product: product.cloned(),
            created_by_account_id: created_by,
            created_by_name: None,
            created_at: Utc::now(),
            done_at: None,
            done_by_account_id: None,
            purchase_id: None,
        };

    // create with product
    let with_product = db
        .create_shopping_list_item(new_item("Mate", Some(&product), Some(member.id)))
        .await
        .unwrap();
    assert!(with_product.id != 0);
    assert_eq!(with_product.name, "Mate");
    assert_eq!(with_product.quantity, 2);
    assert_eq!(with_product.note, "note");
    assert_eq!(with_product.product, Some(product.clone()));
    assert_eq!(with_product.created_by_account_id, Some(member.id));
    assert_eq!(with_product.created_by_name.as_deref(), Some("List Member"));
    assert_eq!(with_product.done_at, None);
    assert_eq!(with_product.done_by_account_id, None);
    assert_eq!(with_product.purchase_id, None);

    // create free text without creator
    let free_text = db
        .create_shopping_list_item(new_item("Dish soap", None, None))
        .await
        .unwrap();
    assert_eq!(free_text.product, None);
    assert_eq!(free_text.created_by_account_id, None);
    assert_eq!(free_text.created_by_name, None);

    assert_eq!(
        db.get_shopping_list_item_by_id(with_product.id)
            .await
            .unwrap(),
        Some(with_product.clone())
    );
    assert_eq!(db.get_shopping_list_item_by_id(424242).await.unwrap(), None);

    // open list: oldest first
    let open = db
        .get_shopping_list_items(ShoppingListState::Open)
        .await
        .unwrap();
    assert_eq!(open, vec![with_product.clone(), free_text.clone()]);
    assert!(db
        .get_shopping_list_items(ShoppingListState::Done)
        .await
        .unwrap()
        .is_empty());

    // update
    let mut changed = with_product.clone();
    changed.name = "Mate crate".to_string();
    changed.quantity = 3;
    changed.product = None;
    let updated = db.update_shopping_list_item(changed).await.unwrap();
    assert_eq!(updated.name, "Mate crate");
    assert_eq!(updated.quantity, 3);
    assert_eq!(updated.product, None);
    let mut missing = updated.clone();
    missing.id = 424242;
    assert_eq!(
        db.update_shopping_list_item(missing).await,
        Err(ServiceError::NotFound)
    );

    // reopen an open item is a conflict
    assert!(matches!(
        db.reopen_shopping_list_item(updated.id).await,
        Err(ServiceError::Conflict(_))
    ));

    // done with unknown purchase
    assert_eq!(
        db.mark_shopping_list_item_done(updated.id, Some(purchaser.id), Some(424242))
            .await,
        Err(ServiceError::NotFound)
    );
    assert_eq!(
        db.mark_shopping_list_item_done(424242, Some(purchaser.id), None)
            .await,
        Err(ServiceError::NotFound)
    );

    // done with purchase
    let purchase = db
        .create_purchase(test_purchase(PurchaseState::Draft, Utc::now(), vec![]))
        .await
        .unwrap();
    let done = db
        .mark_shopping_list_item_done(updated.id, Some(purchaser.id), Some(purchase.id))
        .await
        .unwrap();
    assert!(done.done_at.is_some());
    assert_eq!(done.done_by_account_id, Some(purchaser.id));
    assert_eq!(done.purchase_id, Some(purchase.id));
    assert!(matches!(
        db.mark_shopping_list_item_done(updated.id, Some(purchaser.id), None)
            .await,
        Err(ServiceError::Conflict(_))
    ));
    assert!(matches!(
        db.update_shopping_list_item(done.clone()).await,
        Err(ServiceError::Conflict(_))
    ));

    // second done item, newest done first
    let done2 = db
        .mark_shopping_list_item_done(free_text.id, Some(purchaser.id), None)
        .await
        .unwrap();
    assert_eq!(done2.purchase_id, None);
    let done_list = db
        .get_shopping_list_items(ShoppingListState::Done)
        .await
        .unwrap();
    assert_eq!(done_list, vec![done2.clone(), done.clone()]);
    assert!(db
        .get_shopping_list_items(ShoppingListState::Open)
        .await
        .unwrap()
        .is_empty());

    // reopen clears the done fields and the purchase link
    let reopened = db.reopen_shopping_list_item(done.id).await.unwrap();
    assert_eq!(reopened.done_at, None);
    assert_eq!(reopened.done_by_account_id, None);
    assert_eq!(reopened.purchase_id, None);
    assert_eq!(
        db.reopen_shopping_list_item(424242).await,
        Err(ServiceError::NotFound)
    );

    // all: open entries first, then done
    let all = db
        .get_shopping_list_items(ShoppingListState::All)
        .await
        .unwrap();
    assert_eq!(all, vec![reopened.clone(), done2.clone()]);

    // deleting the purchase keeps the item but unlinks it
    let done = db
        .mark_shopping_list_item_done(reopened.id, Some(purchaser.id), Some(purchase.id))
        .await
        .unwrap();
    assert_eq!(done.purchase_id, Some(purchase.id));
    db.delete_purchase(purchase.id).await.unwrap();
    let unlinked = db
        .get_shopping_list_item_by_id(done.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unlinked.purchase_id, None);
    assert!(unlinked.done_at.is_some());

    // delete open and done items
    assert_eq!(db.delete_shopping_list_item(done2.id).await, Ok(()));
    assert_eq!(db.delete_shopping_list_item(done.id).await, Ok(()));
    assert_eq!(
        db.delete_shopping_list_item(done.id).await,
        Err(ServiceError::NotFound)
    );
    assert!(db
        .get_shopping_list_items(ShoppingListState::All)
        .await
        .unwrap()
        .is_empty());
}
