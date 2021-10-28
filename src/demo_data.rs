use chrono::{Duration, Local, NaiveDateTime, NaiveTime};
use rand::prelude::SliceRandom;

use crate::model::transactions::{execute_at, TransactionItemInput};
use crate::model::{authentication_password, Account, Category, Permission, Product, StampType};
use crate::utils::{DatabaseConnection, DatabasePool, Money, ServiceError, ServiceResult};

fn add_account(
    database_conn: &DatabaseConnection,
    username: &str,
    name: &str,
    permission: Permission,
) -> ServiceResult<Account> {
    let mut account = Account::create(database_conn, name, permission)?;
    account.username = username.to_owned();
    account.update(database_conn)?;
    authentication_password::register(database_conn, &account, "password")?;
    Ok(account)
}

fn add_category(
    database_conn: &DatabaseConnection,
    name: &str,
    price: Money,
    pay_with_stamps: StampType,
    give_stamps: StampType,
) -> ServiceResult<Category> {
    let mut catagory = Category::create(database_conn, name, price)?;
    catagory.pay_with_stamps = pay_with_stamps;
    catagory.give_stamps = give_stamps;
    catagory.update(database_conn)?;
    Ok(catagory)
}

fn add_product(
    database_conn: &DatabaseConnection,
    name: &str,
    barcode: Option<&str>,
    category: &Category,
    price: Option<Money>,
    pay_with_stamps: Option<StampType>,
    give_stamps: Option<StampType>,
) -> ServiceResult<Product> {
    let mut product = Product::create(database_conn, name, category)?;
    product.barcode = barcode.map(|b| b.to_owned());
    product.price = price;
    product.pay_with_stamps = pay_with_stamps;
    product.give_stamps = give_stamps;
    product.update(database_conn)?;
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
            let mut transaction_items: Vec<TransactionItemInput> = Vec::new();
            while price < avg_down.abs() {
                let (p, c) = products.choose(&mut rng).unwrap();

                let pr = p.price.unwrap_or(c.price);
                price += pr;

                let give_stamps = p.give_stamps.unwrap_or(c.give_stamps);

                transaction_items.push(TransactionItemInput {
                    price: -pr,
                    pay_with_stamps: StampType::None,
                    could_be_paid_with_stamps: StampType::None,
                    give_stamps,
                    product_id: Some(p.id),
                });
            }

            while account.credit - price < account.minimum_credit {
                execute_at(
                    database_conn,
                    account,
                    vec![TransactionItemInput {
                        price: avg_up,
                        pay_with_stamps: StampType::None,
                        could_be_paid_with_stamps: StampType::None,
                        give_stamps: StampType::None,
                        product_id: None,
                    }],
                    false,
                    date_time + Duration::seconds(seconds),
                )?;
                seconds += 60;
            }

            let result = execute_at(
                database_conn,
                account,
                transaction_items,
                false,
                date_time + Duration::seconds(seconds),
            );

            match result {
                Ok(_) => {}
                Err(ServiceError::TransactionCancelled(_)) => {
                    // TODO
                }
                Err(e) => {
                    return Err(e);
                }
            }
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
        Permission::Member,
    )?;
    let mut account_default = add_account(
        database_conn,
        "default",
        "Demo Default User",
        Permission::Default,
    )?;

    let other = add_category(database_conn, "Other", 0, StampType::None, StampType::None)?;
    let kaltgetraenke_0_5 = add_category(
        database_conn,
        "Kaltgetränke 0,5l",
        150,
        StampType::Bottle,
        StampType::None,
    )?;
    let kaltgetraenke_0_33 = add_category(
        database_conn,
        "Kaltgetränke 0,33l",
        110,
        StampType::Bottle,
        StampType::None,
    )?;
    let kaltgetraenke_0_33_bio = add_category(
        database_conn,
        "Kaltgetränke 0,33l BIO",
        150,
        StampType::Bottle,
        StampType::None,
    )?;
    let snacks = add_category(
        database_conn,
        "Snacks",
        100,
        StampType::None,
        StampType::None,
    )?;
    let heisgetraenke = add_category(
        database_conn,
        "Heißgetränke",
        100,
        StampType::Coffee,
        StampType::Coffee,
    )?;

    add_product(
        database_conn,
        "Flaschenpfand Stempel",
        None,
        &other,
        Some(0),
        None,
        Some(StampType::Bottle),
    )?;
    add_product(
        database_conn,
        "Kolle Mate",
        Some("4280001274044"),
        &kaltgetraenke_0_5,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Zotrine",
        Some("4280001274006"),
        &kaltgetraenke_0_5,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Flora Mate",
        Some("4260031874056"),
        &kaltgetraenke_0_5,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Premium Cola",
        None,
        &kaltgetraenke_0_5,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Club Mate",
        Some("4029764001807"),
        &kaltgetraenke_0_5,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Club Mate Eistee",
        Some("4029764001814"),
        &kaltgetraenke_0_5,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Club Mate Granat",
        Some("4029764001401"),
        &kaltgetraenke_0_5,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Fritz Limo Orange",
        Some("4260107222989"),
        &kaltgetraenke_0_5,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Fritz-Spritz Apfelschorle",
        Some("4260107222576"),
        &kaltgetraenke_0_5,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Wostok Aprikose Mandel",
        Some("4260189210096"),
        &kaltgetraenke_0_33,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Wostok Dattel Granatapfel",
        Some("4260189210034"),
        &kaltgetraenke_0_33,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Wostok Estragon-Ingwer",
        Some("4260189210058"),
        &kaltgetraenke_0_33,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Wostok Tannenwald",
        Some("4260189210010"),
        &kaltgetraenke_0_33,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Biozisch Matcha",
        Some("4015533025419"),
        &kaltgetraenke_0_33_bio,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Biozisch Ginger Life",
        Some("4015533019586"),
        &kaltgetraenke_0_33_bio,
        None,
        None,
        None,
    )?;
    add_product(
        database_conn,
        "BioZisch Gurke",
        Some("4015533028236"),
        &kaltgetraenke_0_33_bio,
        None,
        None,
        None,
    )?;

    add_product(
        database_conn,
        "Mr. Tom",
        Some("4021700800000"),
        &snacks,
        Some(40),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Die Gute Bio Schokolade",
        Some("7610815028774"),
        &snacks,
        Some(140),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Kinder Riegel",
        Some("40084077"),
        &snacks,
        Some(30),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Kinder Bueno",
        Some("4008400935225"),
        &snacks,
        Some(80),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Snickers",
        Some("5000159461122"),
        &snacks,
        Some(80),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Twix",
        Some("5000159459228"),
        &snacks,
        Some(80),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Knoppers",
        Some("40358802"),
        &snacks,
        Some(60),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Hanuta",
        Some("7120873518730"),
        &snacks,
        Some(60),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Pickup",
        Some("4017100213045"),
        &snacks,
        Some(40),
        None,
        None,
    )?;

    add_product(
        database_conn,
        "Kaffee Stempel",
        None,
        &other,
        Some(0),
        None,
        Some(StampType::Coffee),
    )?;
    add_product(
        database_conn,
        "Kaffee (C)",
        None,
        &heisgetraenke,
        Some(80),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Kaffee Creme (C++)",
        None,
        &heisgetraenke,
        Some(80),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Cafe americano (Rust)",
        None,
        &heisgetraenke,
        Some(80),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Kaffee (C--)",
        None,
        &heisgetraenke,
        Some(80),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Espresso (Lua)",
        None,
        &heisgetraenke,
        Some(80),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Espresso Doppio (Julia)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Ristretto (Schema)",
        None,
        &heisgetraenke,
        Some(80),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Espresso Macchiato (Erlang)",
        None,
        &heisgetraenke,
        Some(80),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Melange (Go)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Milchkaffee (Java)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Cappuccino (Python)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Latte Macciato (Ruby)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Großer Latte Macciato (Rubinious)",
        None,
        &heisgetraenke,
        Some(120),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Chocolate (bash)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "White Chocolate (Javascript)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Chocolate Espresso (ObjectiveC)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "White Chocolate Espresso (Swift)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Chococcino (Perl)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
    )?;
    add_product(
        database_conn,
        "Frotty Bash (fish)",
        None,
        &heisgetraenke,
        Some(100),
        None,
        None,
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
