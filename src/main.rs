#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate handlebars;
extern crate rpassword;
extern crate uuid;

use diesel::r2d2::{self, ConnectionManager};

use std::io::{stdin, stdout, Write};

mod api;
mod core;
mod server;
mod web;

use crate::core::{authentication_password, Account, DbConnection, Pool, ServiceError};
use server::start_server;

// For later ref: https://gill.net.in/posts/auth-microservice-rust-actix-web1.0-diesel-complete-tutorial/
fn main() -> Result<(), ServiceError> {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=info");
    env_logger::init();

    // Setup database connection
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<DbConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    // Check if admin exists, create otherwise
    check_admin(&pool);

    // Setup web server
    start_server(pool)?;

    Ok(())
}

fn read_value(prompt: &str, hide_input: bool) -> String {
    if hide_input {
        loop {
            let p1 = rpassword::prompt_password_stdout(prompt).unwrap();
            let p2 = rpassword::prompt_password_stdout(prompt).unwrap();

            if p1 == p2 {
                return p1;
            } else {
                println!("Passwords does not match, retry.");
            }
        }
    } else {
        print!("{}", prompt);
        stdout().flush().unwrap();
        let mut value = String::new();
        stdin().read_line(&mut value).unwrap();
        return value.trim().to_owned();
    }
}

fn check_admin(pool: &Pool) {
    let conn = &pool.get().unwrap();
    if Account::all(&conn).unwrap().is_empty() {
        let fullname = read_value("Fullname: ", false);
        let username = read_value("Username: ", false);
        let password = read_value("Password: ", true);

        let mut account = Account::create(&conn).unwrap();
        account.name = Some(fullname.into());
        account.update(&conn).unwrap();
        authentication_password::register(&conn, &account, &username, &password).unwrap();
    }
}
