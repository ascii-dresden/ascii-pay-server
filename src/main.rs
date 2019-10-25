#[macro_use]
extern crate diesel;
extern crate uuid;

mod core;

use diesel::prelude::*;

use crate::core::{authentication_password, transactions, Account, Error};

// For later ref: https://gill.net.in/posts/auth-microservice-rust-actix-web1.0-diesel-complete-tutorial/
fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=info");
    env_logger::init();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    if Account::all(&conn)?.is_empty() {
        let mut account = Account::create(&conn)?;
        account.name = Some("Max Mustermann".into());
        account.update(&conn)?;

        authentication_password::register(&conn, &account, "max", "mustermann")?;
    }
    let mut account = authentication_password::get(&conn, "max", "mustermann")?;

    transactions::execute(&conn, &mut account, None, 100)?;
    println!("{:?}", &account);

    let trans = transactions::get_by_user(
        &conn,
        &account,
        &(chrono::Local::now().naive_local() - chrono::Duration::hours(24)),
        &chrono::Local::now().naive_local(),
    )?;

    for t in trans {
        println!("- {:?}", t);
    }

    println!("{:?}", transactions::validate_all(&conn)?);

    Ok(())
}
