use std::collections::HashMap;

use chrono::{Duration, Local, NaiveDateTime, NaiveTime};
use rand::prelude::SliceRandom;

use crate::model::transactions::execute_at;
use crate::model::{authentication_password, Account, Category, Permission, Product};
use crate::utils::{DatabaseConnection, DatabasePool, Money, ServiceResult};

fn add_account(
    database_conn: &DatabaseConnection,
    username: &str,
    name: &str,
    permission: Permission,
) -> ServiceResult<Account> {
    let mut account = Account::create(database_conn, name, permission)?;
    account.username = Some(username.to_owned());
    account.update(database_conn)?;
    authentication_password::register(database_conn, &account, "password")?;
    Ok(account)
}

fn add_category(
    database_conn: &DatabaseConnection,
    name: &str,
    price: Option<Money>,
) -> ServiceResult<Category> {
    let mut catagory = Category::create(database_conn, name)?;
    if let Some(price) = price {
        catagory.add_price(database_conn, NaiveDateTime::from_timestamp(0, 0), price)?;
    }
    Ok(catagory)
}

fn add_product(
    database_conn: &DatabaseConnection,
    name: &str,
    barcode: Option<&str>,
    category: Option<&Category>,
    price: Option<Money>,
) -> ServiceResult<Product> {
    let mut product = Product::create(database_conn, name, category.cloned())?;
    product.barcode = barcode.map(|b| b.to_owned());
    product.update(database_conn)?;
    if let Some(price) = price {
        product.add_price(database_conn, NaiveDateTime::from_timestamp(0, 0), price)?;
    }
    Ok(product)
}

fn generate_transactions(
    database_conn: &DatabaseConnection,
    account: &mut Account,
    from: NaiveDateTime,
    to: NaiveDateTime,
    count_per_day: u32,
    avg_down: Money,
    avg_up: Money,
) -> ServiceResult<()> {
    let days = (to - from).num_days();
    let start_date = from.date();

    let products = Product::all(database_conn)?;
    let mut rng = rand::thread_rng();

    for day_offset in 0..days {
        let offset = Duration::days(day_offset);
        let date = start_date + offset;

        for time_offset in 0..count_per_day {
            let offset = 9.0 / ((count_per_day - 1) as f32) * time_offset as f32;

            let hr = offset as u32;
            let mn = ((offset - hr as f32) * 60.0) as u32;

            let time = NaiveTime::from_hms(9 + hr, mn, 0);

            let date_time = NaiveDateTime::new(date, time);

            let mut seconds = 0;

            let mut price = 0;
            let mut transaction_products: HashMap<Product, i32> = HashMap::new();
            while price < avg_down.abs() {
                let p = products.choose(&mut rng);

                if let Some(p) = p {
                    let pr = p.current_price;

                    if let Some(pr) = pr {
                        price += pr;
                    }

                    let amount = transaction_products.get(p).copied().unwrap_or(0) + 1;
                    transaction_products.insert(p.clone(), amount);
                } else {
                    price = avg_down;
                }
            }

            while account.credit - price < account.minimum_credit {
                execute_at(
                    database_conn,
                    account,
                    None,
                    avg_up,
                    date_time + Duration::seconds(seconds),
                )?;
                seconds += 1;
            }

            let transaction = execute_at(
                database_conn,
                account,
                None,
                -price,
                date_time + Duration::seconds(seconds),
            )?;

            transaction.add_products(
                database_conn,
                transaction_products
                    .into_iter()
                    .map(|(k, v)| (k, v))
                    .collect(),
            )?;
        }
    }

    Ok(())
}

pub fn load_demo_data(database_pool: &DatabasePool) -> ServiceResult<()> {
    let database_conn = &database_pool.get()?;

    let mut account_admin =
        add_account(database_conn, "admin", "Demo Admin User", Permission::Admin)?;
    let mut account_member = add_account(
        database_conn,
        "member",
        "Demo Member User",
        Permission::Admin,
    )?;
    let mut account_default = add_account(
        database_conn,
        "default",
        "Demo Default User",
        Permission::Admin,
    )?;

    let kaltgetraenke_0_5 = add_category(database_conn, "Kaltgetränke 0,5l", Some(150))?;
    let kaltgetraenke_0_33 = add_category(database_conn, "Kaltgetränke 0,33l", Some(110))?;
    let kaltgetraenke_0_33_bio = add_category(database_conn, "Kaltgetränke 0,33l BIO", Some(150))?;
    let _heisgetraenke = add_category(database_conn, "Heißgetränke", Some(100))?;
    let snacks = add_category(database_conn, "Snacks", None)?;

    add_product(
        database_conn,
        "Kolle Mate",
        Some("4280001274044"),
        Some(&kaltgetraenke_0_5),
        None,
    )?;
    add_product(
        database_conn,
        "Zotrine",
        Some("4280001274006"),
        Some(&kaltgetraenke_0_5),
        None,
    )?;
    add_product(
        database_conn,
        "Flora Mate",
        Some("4260031874056"),
        Some(&kaltgetraenke_0_5),
        None,
    )?;
    add_product(
        database_conn,
        "Premium Cola",
        None,
        Some(&kaltgetraenke_0_5),
        None,
    )?;
    add_product(
        database_conn,
        "Club Mate",
        Some("4029764001807"),
        Some(&kaltgetraenke_0_5),
        None,
    )?;
    add_product(
        database_conn,
        "Club Mate Eistee",
        Some("4029764001814"),
        Some(&kaltgetraenke_0_5),
        None,
    )?;
    add_product(
        database_conn,
        "Club Mate Granat",
        Some("4029764001401"),
        Some(&kaltgetraenke_0_5),
        None,
    )?;
    add_product(
        database_conn,
        "Fritz Limo Orange",
        Some("4260107222989"),
        Some(&kaltgetraenke_0_5),
        None,
    )?;
    add_product(
        database_conn,
        "Fritz-Spritz Apfelschorle",
        Some("4260107222576"),
        Some(&kaltgetraenke_0_5),
        None,
    )?;
    add_product(
        database_conn,
        "Wostok Aprikose Mandel",
        Some("4260189210096"),
        Some(&kaltgetraenke_0_33),
        None,
    )?;
    add_product(
        database_conn,
        "Wostok Dattel Granatapfel",
        Some("4260189210034"),
        Some(&kaltgetraenke_0_33),
        None,
    )?;
    add_product(
        database_conn,
        "Wostok Estragon-Ingwer",
        Some("4260189210058"),
        Some(&kaltgetraenke_0_33),
        None,
    )?;
    add_product(
        database_conn,
        "Wostok Tannenwald",
        Some("4260189210010"),
        Some(&kaltgetraenke_0_33),
        None,
    )?;
    add_product(
        database_conn,
        "Biozisch Matcha",
        Some("4015533025419"),
        Some(&kaltgetraenke_0_33_bio),
        None,
    )?;
    add_product(
        database_conn,
        "Biozisch Ginger Life",
        Some("4015533019586"),
        Some(&kaltgetraenke_0_33_bio),
        None,
    )?;
    add_product(
        database_conn,
        "BioZisch Gurke",
        Some("4015533028236"),
        Some(&kaltgetraenke_0_33_bio),
        None,
    )?;

    add_product(
        database_conn,
        "Mr. Tom",
        Some("4021700800000"),
        Some(&snacks),
        Some(40),
    )?;
    add_product(
        database_conn,
        "Die Gute Bio Schokolade",
        Some("7610815028774"),
        Some(&snacks),
        Some(140),
    )?;
    add_product(
        database_conn,
        "Kinder Riegel",
        Some("40084077"),
        Some(&snacks),
        Some(30),
    )?;
    add_product(
        database_conn,
        "Kinder Bueno",
        Some("4008400935225"),
        Some(&snacks),
        Some(80),
    )?;
    add_product(
        database_conn,
        "Snickers",
        Some("5000159461122"),
        Some(&snacks),
        Some(80),
    )?;
    add_product(
        database_conn,
        "Twix",
        Some("5000159459228"),
        Some(&snacks),
        Some(80),
    )?;
    add_product(
        database_conn,
        "Knoppers",
        Some("40358802"),
        Some(&snacks),
        Some(60),
    )?;
    add_product(
        database_conn,
        "Hanuta",
        Some("7120873518730"),
        Some(&snacks),
        Some(60),
    )?;
    add_product(
        database_conn,
        "Pickup",
        Some("4017100213045"),
        Some(&snacks),
        Some(40),
    )?;

    let now = Local::now().naive_local();

    generate_transactions(
        database_conn,
        &mut account_admin,
        now - Duration::days(90),
        now,
        3,
        150,
        2000,
    )?;
    generate_transactions(
        database_conn,
        &mut account_member,
        now - Duration::days(60),
        now,
        2,
        150,
        2000,
    )?;
    generate_transactions(
        database_conn,
        &mut account_default,
        now - Duration::days(30),
        now,
        1,
        150,
        2000,
    )?;

    Ok(())
}
