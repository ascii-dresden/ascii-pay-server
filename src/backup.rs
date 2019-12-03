use crate::core::{
    transactions, Account, Category, DbConnection, Pool, Product, ServiceResult, Transaction,
};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct BackupTransaction {
    transaction: Transaction,
    products: HashMap<Uuid, i32>,
}

impl BackupTransaction {
    fn load(conn: &DbConnection, transaction: Transaction) -> ServiceResult<BackupTransaction> {
        let products = transaction
            .get_products(conn)?
            .into_iter()
            .map(|(key, value)| (key.id, value))
            .collect();

        Ok(BackupTransaction {
            transaction,
            products,
        })
    }
}

fn write_data<W, S>(tar: &mut tar::Builder<W>, path: &str, content: &S) -> ServiceResult<()>
where
    W: Write,
    S: Serialize,
{
    let name = Uuid::new_v4()
        .to_hyphenated()
        .encode_upper(&mut Uuid::encode_buffer())
        .to_string();

    let mut dir = std::env::temp_dir();
    dir.push(format!("aps-{}.tmp", name));

    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(&dir)?;

    serde_json::to_writer_pretty(&file, content)?;
    file.flush()?;

    tar.append_path_with_name(&dir, path)?;

    fs::remove_file(&dir)?;

    Ok(())
}

fn read_data<D>(dir: &PathBuf, name: &str) -> ServiceResult<D>
where
    D: DeserializeOwned,
{
    let mut dir = dir.clone();
    dir.push(name);

    let file = File::open(&dir)?;
    Ok(serde_json::from_reader(file)?)
}

pub fn export(pool: &Pool, dest: &str) -> ServiceResult<()> {
    let tar_gz = File::create(dest)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);

    let conn = pool.get()?;

    write_data(&mut tar, "backup/accounts.json", &Account::all(&conn)?)?;
    write_data(&mut tar, "backup/products.json", &Product::all(&conn)?)?;
    write_data(&mut tar, "backup/categories.json", &Category::all(&conn)?)?;

    let transactions: Vec<BackupTransaction> = Transaction::all(&conn)?
        .into_iter()
        .map(|t| BackupTransaction::load(&conn, t).unwrap())
        .collect();
    write_data(&mut tar, "backup/transactions.json", &transactions)?;

    let images: HashSet<String> = Product::all(&conn)?
        .into_iter()
        .filter_map(|p| p.image)
        .collect();

    for image in images {
        tar.append_path_with_name(format!("backup/img/{}", image), format!("img/{}", image))?;
    }

    tar.finish()?;

    Ok(())
}

pub fn import(pool: &Pool, src: &str) -> ServiceResult<()> {
    let tar_gz = File::open(src)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);

    let conn = pool.get()?;

    let name = Uuid::new_v4()
        .to_hyphenated()
        .encode_upper(&mut Uuid::encode_buffer())
        .to_string();

    let mut dir = std::env::temp_dir();
    dir.push(format!("aps-{}", name));
    fs::create_dir(&dir)?;
    archive.unpack(&dir)?;

    let accounts: Vec<Account> = read_data(&dir, "backup/accounts.json")?;
    let mut account_map: HashMap<Uuid, Account> = HashMap::new();
    for account in accounts {
        account_map.insert(account.id, Account::import(&conn, &account)?);
    }

    let categories: Vec<Category> = read_data(&dir, "backup/categories.json")?;
    let mut category_map: HashMap<Uuid, Category> = HashMap::new();
    for category in categories {
        category_map.insert(category.id, Category::import(&conn, &category)?);
    }

    let products: Vec<Product> = read_data(&dir, "backup/products.json")?;
    let mut product_map: HashMap<Uuid, Product> = HashMap::new();
    for product in products {
        let category = if let Some(old_category) = &product.category {
            if let Some(new_category) = category_map.get(&old_category.id) {
                Some(new_category.clone())
            } else {
                None
            }
        } else {
            None
        };

        product_map.insert(product.id, Product::import(&conn, &product, category)?);
    }

    let transaction_list: Vec<BackupTransaction> = read_data(&dir, "backup/transactions.json")?;
    for backup_transaction in transaction_list {
        let cashier = backup_transaction
            .transaction
            .cashier_id
            .map(|id| account_map[&id].clone());
        let account = account_map
            .get_mut(&backup_transaction.transaction.account_id)
            .unwrap();

        let trans = transactions::import(
            &conn,
            account,
            cashier.as_ref(),
            backup_transaction.transaction.total,
            backup_transaction.transaction.date,
        )?;

        let transaction_products: HashMap<Product, i32> = backup_transaction
            .products
            .iter()
            .map(|(key, count)| {
                let product = product_map.get(key).unwrap().clone();
                (product, *count)
            })
            .collect();
        trans.add_products(&conn, transaction_products)?;
    }

    fs::remove_dir_all(&dir)?;

    Ok(())
}
