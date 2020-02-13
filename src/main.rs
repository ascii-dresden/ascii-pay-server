#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate block_modes;
extern crate clap;
extern crate handlebars;
extern crate rpassword;
extern crate uuid;
#[macro_use]
extern crate hex_literal;

use clap::App;
use diesel::r2d2::{self, ConnectionManager};

use std::io::{stdin, stdout, Write};

mod api;
mod core;
mod identity_policy;
mod server;
mod web;

use crate::core::{
    authentication_password, env, Account, DbConnection, Permission, Pool, ServiceResult,
};
use server::start_server;

#[actix_rt::main]
async fn main() -> ServiceResult<()> {
    let result = init().await;

    let exit_code = match result {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    };

    std::process::exit(exit_code);
}

// For later ref: https://gill.net.in/posts/auth-microservice-rust-actix-web1.0-diesel-complete-tutorial/
async fn init() -> ServiceResult<()> {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=info");
    env_logger::init();

    // Setup database connection
    let manager = ConnectionManager::<DbConnection>::new(env::DATABASE_URL.as_str());
    let pool = r2d2::Pool::builder().build(manager)?;

    let _matches = App::new("ascii-pay")
        .version("1.0")
        .author("Lars Westermann <lars-westermann@live.de>")
        .author("Felix Wittwer <dev@felixwittwer.de>")
        .get_matches();

    // Check if admin exists, create otherwise
    check_admin_user_exisits(&pool)?;

    // Setup web server
    start_server(pool).await?;

    Ok(())
}

/// Read a value from stdin
///
/// # Arguments
/// * `prompt` - A prompt that descripes the required input
/// * `hide_input` - Specifies if the input value is visible or hidden
fn read_value(prompt: &str, hide_input: bool) -> String {
    if hide_input {
        loop {
            let p1 = rpassword::prompt_password_stdout(prompt).unwrap();
            let p2 = rpassword::prompt_password_stdout(prompt).unwrap();

            if p1 == p2 {
                return p1;
            } else {
                println!("Passwords do not match, retry.");
            }
        }
    } else {
        print!("{}", prompt);
        stdout().flush().unwrap();
        let mut value = String::new();
        stdin().read_line(&mut value).unwrap();
        value.trim().to_owned()
    }
}

/// Check if a initial user exists. Otherwise create a new one
fn check_admin_user_exisits(pool: &Pool) -> ServiceResult<()> {
    let conn = &pool.get().unwrap();

    let admin_with_password_exists = Account::all(&conn)?
        .iter()
        .filter(|a| a.permission.is_admin())
        .any(|a| authentication_password::has_password(&conn, a).unwrap_or(false));

    if !admin_with_password_exists {
        println!("You seem to have started the server on an empty database. We'll now create the initial superuser.");
        let fullname = read_value("Fullname: ", false);
        let username = read_value("Username: ", false);
        let password = read_value("Password: ", true);

        let mut account = Account::create(&conn, &fullname, Permission::ADMIN)?;
        account.username = Some(username);
        account.update(&conn)?;
        authentication_password::register(&conn, &account, &password)?;
    }

    Ok(())
}
