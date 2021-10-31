use chrono::{Duration, Local, NaiveDateTime, NaiveTime};
use rand::prelude::SliceRandom;

use crate::model::transactions::{execute_at, TransactionItemInput};
use crate::model::{authentication_password, Account, Permission, Product, StampType};
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

    let products = Product::all()?;
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
                let p = products.choose(&mut rng).unwrap();

                let pr = p.price;
                price += pr;

                let give_stamps = p.give_stamps;

                transaction_items.push(TransactionItemInput {
                    price: -pr,
                    pay_with_stamps: StampType::None,
                    could_be_paid_with_stamps: StampType::None,
                    give_stamps,
                    product_id: p.id.clone(),
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
                        product_id: String::new(),
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
