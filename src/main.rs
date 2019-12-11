#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate clap;
extern crate flate2;
extern crate handlebars;
extern crate rpassword;
extern crate tar;
extern crate uuid;

use clap::{App, Arg};
use diesel::r2d2::{self, ConnectionManager};

use std::io::{stdin, stdout, Write};

mod api;
mod backup;
mod core;
mod server;
mod web;

use crate::core::{
    authentication_password, Account, DbConnection, Permission, Pool, ServiceResult,
};
use server::start_server;

// For later ref: https://gill.net.in/posts/auth-microservice-rust-actix-web1.0-diesel-complete-tutorial/
fn main() -> ServiceResult<()> {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=info");
    env_logger::init();

    // Setup database connection
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<DbConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let matches = App::new("ascii-prepaid-system")
        .version("1.0")
        .author("Lars Westermann <lars-westermann@live.de>")
        .author("Felix Wittwer <dev@felixwittwer.de>")
        .arg(
            Arg::with_name("export")
                .long("export")
                .value_name("FILE")
                .help("Exports the database to the given file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("import")
                .long("import")
                .value_name("FILE")
                .help("Import the given file to the database")
                .takes_value(true),
        )
        .get_matches();

    if let Some(export_file) = matches.value_of("export") {
        return backup::export(&pool, export_file);
    }
    if let Some(import_file) = matches.value_of("import") {
        return backup::import(&pool, import_file);
    }

    // Check if admin exists, create otherwise
    check_admin(&pool)?;

    // Setup web server
    start_server(pool)?;

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
fn check_admin(pool: &Pool) -> ServiceResult<()> {
    let conn = &pool.get().unwrap();

    let admin_with_password_exists = Account::all(&conn)?
        .iter()
        .filter(|a| a.permission.is_admin())
        .any(|a| {
            !authentication_password::get_usernames(&conn, a)
                .unwrap_or_else(|_| vec![])
                .is_empty()
        });

    if !admin_with_password_exists {
        println!("You seem to have started the server on an empty database. We'll now create the initial superuser.");
        let fullname = read_value("Fullname: ", false);
        let username = read_value("Username: ", false);
        let password = read_value("Password: ", true);

        let mut account = Account::create(&conn, Permission::ADMIN)?;
        account.name = Some(fullname);
        account.update(&conn)?;
        authentication_password::register(&conn, &account, &username, &password)?;
    }

    Ok(())
}
