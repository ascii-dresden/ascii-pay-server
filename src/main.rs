#[macro_use]
extern crate diesel;
extern crate uuid;

mod core;

use diesel::prelude::*;

use crate::core::{Account, Error, authentication_password};

// For later ref: https://gill.net.in/posts/auth-microservice-rust-actix-web1.0-diesel-complete-tutorial/
fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=info");
    env_logger::init();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    let accounts = Account::all(&conn)?;

    if accounts.is_empty() {
        let mut account = Account::create(&conn)?;
        account.name = Some("Max Mustermann".into());
        account.update(&conn)?;

        authentication_password::register(&conn, &account, "max", "mustermann")?;
    } else {
        for mut account in accounts {
            account.credit += 1;
            account.update(&conn)?;
            println!("List accounts: {:?}", account);
        }
    }

    let found = authentication_password::get(&conn, "max", "mustermann")?;
    println!("Test login: {:?}", found);

    Ok(())
}
