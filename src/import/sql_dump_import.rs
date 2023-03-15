use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
};

use chrono::{TimeZone, Utc};

use crate::{
    database::DatabaseConnection,
    error::ServiceResult,
    models::{Account, AuthNfc, AuthPassword, CoinAmount, CoinType, Payment, PaymentItem},
};

pub async fn load_sql_dump_into_database(
    db: &mut DatabaseConnection,
    path: &str,
    products_map: HashMap<String, u64>,
) -> ServiceResult<()> {
    let data = parse_sql_file(path);

    let data_account = parse_accounts(&data);
    let data_authentication_password = parse_authentication_password(&data);
    let data_authentication_nfc = parse_authentication_nfc(&data);
    let data_transactions = parse_transaction(&data);
    let data_transaction_items = parse_transaction_items(&data);

    for account_row in data_account {
        let password = data_authentication_password
            .get(&account_row.id)
            .cloned()
            .unwrap_or_default();
        let nfc = data_authentication_nfc
            .get(&account_row.id)
            .cloned()
            .unwrap_or_default();
        let transactions = data_transactions
            .get(&account_row.id)
            .cloned()
            .unwrap_or_default();

        let mut auth_methods = Vec::new();

        for item in password {
            auth_methods.push(crate::models::AuthMethod::PasswordBased(AuthPassword {
                username: account_row.username.clone(),
                password_hash: map_password(&item.password),
            }));
        }

        for item in nfc {
            auth_methods.push(crate::models::AuthMethod::NfcBased(AuthNfc {
                name: item.name,
                card_id: str_to_bytes(&item.card_id),
                card_type: match item.card_type.as_str() {
                    "mifare-desfire" => crate::models::CardType::AsciiMifare,
                    _ => crate::models::CardType::GenericNfc,
                },
                data: str_to_bytes(&item.data),
            }));
        }

        let mut new_account = db
            .store_account(Account {
                id: 0,
                name: account_row.name,
                email: account_row.mail,
                role: match account_row.permission {
                    1 => crate::models::Role::Member,
                    2 => crate::models::Role::Admin,
                    _ => crate::models::Role::Basic,
                },
                balance: CoinAmount::zero(),
                auth_methods,
            })
            .await?;

        for transaction in transactions {
            let mut cents = -transaction.total;
            let mut bottle_stamps = -transaction.bottle_stamps;
            let mut coffee_stamps = -transaction.coffee_stamps;

            let mut payment_items: Vec<PaymentItem> = Vec::new();

            let transaction_items = data_transaction_items
                .get(&transaction.id)
                .cloned()
                .unwrap_or_default();

            for transaction_item in transaction_items {
                let product_id = if !transaction_item.product_id.is_empty() {
                    products_map.get(&transaction_item.product_id).copied()
                } else {
                    None
                };

                let mut amount = HashMap::new();

                if transaction_item.price != 0 {
                    amount.insert(CoinType::Cent, -transaction_item.price);
                    cents += transaction_item.price;
                }

                if transaction_item.pay_with_stamps == 1 {
                    amount.insert(CoinType::CoffeeStamp, 10);
                    coffee_stamps -= 10;
                }
                if transaction_item.pay_with_stamps == 2 {
                    amount.insert(CoinType::BottleStamp, 10);
                    bottle_stamps -= 10;
                }

                if transaction_item.give_stamps == 1 {
                    amount.insert(CoinType::CoffeeStamp, -1);
                    coffee_stamps += 1;
                }
                if transaction_item.give_stamps == 2 {
                    amount.insert(CoinType::BottleStamp, -1);
                    bottle_stamps += 1;
                }

                payment_items.push(PaymentItem {
                    effective_price: CoinAmount(amount),
                    product_id,
                });
            }

            if cents != 0 || bottle_stamps != 0 || coffee_stamps != 0 {
                let mut amount = HashMap::new();

                if cents != 0 {
                    amount.insert(CoinType::Cent, cents);
                }
                if bottle_stamps != 0 {
                    amount.insert(CoinType::BottleStamp, bottle_stamps);
                }
                if coffee_stamps != 0 {
                    amount.insert(CoinType::CoffeeStamp, coffee_stamps);
                }

                payment_items.push(PaymentItem {
                    effective_price: CoinAmount(amount),
                    product_id: None,
                });
            }

            db.payment(
                Payment {
                    account: new_account.id,
                    items: payment_items,
                },
                Utc.datetime_from_str(&transaction.date, "%Y-%m-%d %H:%M:%S.%f")
                    .unwrap(),
            )
            .await?;

            new_account = db.get_account_by_id(new_account.id).await?.unwrap();
        }
    }

    Ok(())
}

fn str_to_bytes(data: &str) -> Vec<u8> {
    if data.is_empty() || data == "none" {
        return Vec::new();
    }
    let r = data.replace(':', " ");
    r.split(' ')
        .map(|x| u8::from_str_radix(x, 16).unwrap_or(0))
        .collect()
}

fn map_password(password: &str) -> Vec<u8> {
    password.as_bytes().into()
}

fn parse_sql_file(path: &str) -> HashMap<String, Vec<Vec<String>>> {
    let mut data: HashMap<String, Vec<Vec<String>>> = HashMap::new();

    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);

    let mut header: Option<String> = None;
    let mut data_entries: Vec<Vec<String>> = Vec::new();

    for line in reader.lines() {
        let line = line.unwrap();

        if line.starts_with("COPY public.") {
            let split = line.replace("COPY public.", "");
            let name = split.split(' ').next().unwrap_or("");
            header = Some(name.to_string());
            data_entries = Vec::new();
            continue;
        }

        if let Some(ref name) = header {
            if line == r"\." {
                data.insert(name.to_string(), data_entries);
                header = None;
                data_entries = Vec::new();
                continue;
            }
        }

        if header.is_some() {
            let split = line.split('\t');
            data_entries.push(split.map(|e| e.to_string()).collect())
        }
    }

    data
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct AccountRow {
    pub id: String,
    pub credit: i32,
    pub minimum_credit: i32,
    pub name: String,
    pub mail: String,
    pub username: String,
    pub account_number: String,
    pub permission: i32,
    pub use_digital_stamps: bool,
    pub coffee_stamps: i32,
    pub bottle_stamps: i32,
    pub receives_monthly_report: bool,
}

impl From<Vec<String>> for AccountRow {
    fn from(row: Vec<String>) -> AccountRow {
        AccountRow {
            id: row.get(0).cloned().unwrap_or_default(),
            credit: row
                .get(1)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            minimum_credit: row
                .get(2)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            name: row.get(3).cloned().unwrap_or_default(),
            mail: row.get(4).cloned().unwrap_or_default(),
            username: row.get(5).cloned().unwrap_or_default(),
            account_number: row.get(6).cloned().unwrap_or_default(),
            permission: row
                .get(7)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            use_digital_stamps: row[8] == "t",
            coffee_stamps: row
                .get(9)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            bottle_stamps: row
                .get(10)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            receives_monthly_report: row[11] == "t",
        }
    }
}

fn parse_accounts(data: &HashMap<String, Vec<Vec<String>>>) -> Vec<AccountRow> {
    data["account"]
        .clone()
        .into_iter()
        .map(AccountRow::from)
        .collect()
}

#[derive(Debug, Clone)]
struct AuthenticationPasswordRow {
    pub account_id: String,
    pub password: String,
}

impl From<Vec<String>> for AuthenticationPasswordRow {
    fn from(row: Vec<String>) -> AuthenticationPasswordRow {
        AuthenticationPasswordRow {
            account_id: row.get(0).cloned().unwrap_or_default(),
            password: row.get(1).cloned().unwrap_or_default(),
        }
    }
}

fn parse_authentication_password(
    data: &HashMap<String, Vec<Vec<String>>>,
) -> HashMap<String, Vec<AuthenticationPasswordRow>> {
    let transactions: Vec<AuthenticationPasswordRow> = data["authentication_password"]
        .clone()
        .into_iter()
        .map(AuthenticationPasswordRow::from)
        .collect();

    let mut map = HashMap::new();
    for item in transactions {
        map.entry(item.account_id.clone())
            .or_insert(vec![])
            .push(item);
    }
    map
}

#[derive(Debug, Clone)]
struct AuthenticationNfcRow {
    pub account_id: String,
    pub card_id: String,
    pub card_type: String,
    pub name: String,
    pub data: String,
}

impl From<Vec<String>> for AuthenticationNfcRow {
    fn from(row: Vec<String>) -> AuthenticationNfcRow {
        AuthenticationNfcRow {
            account_id: row.get(0).cloned().unwrap_or_default(),
            card_id: row.get(1).cloned().unwrap_or_default(),
            card_type: row.get(2).cloned().unwrap_or_default(),
            name: row.get(3).cloned().unwrap_or_default(),
            data: row.get(4).cloned().unwrap_or_default(),
        }
    }
}

fn parse_authentication_nfc(
    data: &HashMap<String, Vec<Vec<String>>>,
) -> HashMap<String, Vec<AuthenticationNfcRow>> {
    let transactions: Vec<AuthenticationNfcRow> = data["authentication_nfc"]
        .clone()
        .into_iter()
        .map(AuthenticationNfcRow::from)
        .collect();

    let mut map = HashMap::new();
    for item in transactions {
        map.entry(item.account_id.clone())
            .or_insert(vec![])
            .push(item);
    }
    map
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct TransactionRow {
    pub id: String,
    pub account_id: String,
    pub total: i32,
    pub before_credit: i32,
    pub after_credit: i32,
    pub coffee_stamps: i32,
    pub before_coffee_stamps: i32,
    pub after_coffee_stamps: i32,
    pub bottle_stamps: i32,
    pub before_bottle_stamps: i32,
    pub after_bottle_stamps: i32,
    pub date: String,
}

impl From<Vec<String>> for TransactionRow {
    fn from(row: Vec<String>) -> TransactionRow {
        TransactionRow {
            id: row.get(0).cloned().unwrap_or_default(),
            account_id: row.get(1).cloned().unwrap_or_default(),
            total: row
                .get(2)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            before_credit: row
                .get(3)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            after_credit: row
                .get(4)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            coffee_stamps: row
                .get(5)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            before_coffee_stamps: row
                .get(6)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            after_coffee_stamps: row
                .get(7)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            bottle_stamps: row
                .get(8)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            before_bottle_stamps: row
                .get(9)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            after_bottle_stamps: row
                .get(10)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            date: row.get(11).cloned().unwrap_or_default(),
        }
    }
}

fn parse_transaction(
    data: &HashMap<String, Vec<Vec<String>>>,
) -> HashMap<String, Vec<TransactionRow>> {
    let transactions: Vec<TransactionRow> = data["transaction"]
        .clone()
        .into_iter()
        .map(TransactionRow::from)
        .collect();

    let mut map = HashMap::new();
    for item in transactions {
        map.entry(item.account_id.clone())
            .or_insert(vec![])
            .push(item);
    }
    map
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct TransactionItemRow {
    pub transaction_id: String,
    pub index: i32,
    pub price: i32,
    pub pay_with_stamps: i32,
    pub give_stamps: i32,
    pub product_id: String,
}

impl From<Vec<String>> for TransactionItemRow {
    fn from(row: Vec<String>) -> TransactionItemRow {
        TransactionItemRow {
            transaction_id: row.get(0).cloned().unwrap_or_default(),
            index: row
                .get(1)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            price: row
                .get(2)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            pay_with_stamps: row
                .get(3)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            give_stamps: row
                .get(4)
                .cloned()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or(0),
            product_id: row.get(5).cloned().unwrap_or_default(),
        }
    }
}

fn parse_transaction_items(
    data: &HashMap<String, Vec<Vec<String>>>,
) -> HashMap<String, Vec<TransactionItemRow>> {
    let items: Vec<TransactionItemRow> = data["transaction_item"]
        .clone()
        .into_iter()
        .map(TransactionItemRow::from)
        .collect();

    let mut map = HashMap::new();
    for item in items {
        map.entry(item.transaction_id.clone())
            .or_insert(vec![])
            .push(item);
    }
    map
}
