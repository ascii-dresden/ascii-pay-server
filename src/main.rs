#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate block_modes;
#[macro_use]
extern crate clap;
extern crate handlebars;
extern crate uuid;
#[macro_use]
extern crate hex_literal;
extern crate rpassword;
extern crate wallet_pass;
#[macro_use]
extern crate async_graphql;

use std::io::Write;

use clap::{App, SubCommand};
use diesel::r2d2::{self, ConnectionManager};

// Internal services
mod identity_service;
mod model;
mod repo;
mod server;

// endpoints
mod api;

use crate::model::{
    authentication_password, env, Account, DbConnection, Permission, Pool, ServiceResult,
};
use server::start_server;

embed_migrations!();

#[actix_web::main]
async fn main() {
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

async fn init() -> ServiceResult<()> {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=info");
    env_logger::init();

    // Setup database connection
    let manager = ConnectionManager::<DbConnection>::new(env::DATABASE_URL.as_str());
    let pool = r2d2::Pool::builder().build(manager)?;

    let conn = pool.get()?;
    embedded_migrations::run_with_output(&conn, &mut std::io::stdout())?;

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .author(crate_authors!("\n"))
        .subcommand(SubCommand::with_name("run").about("Start the web server"))
        .subcommand(SubCommand::with_name("admin").about("Create a new admin user"))
        .get_matches();

    if let Some(_matches) = matches.subcommand_matches("run") {
        // Setup web server
        start_server(pool).await?;
        return Ok(());
    }

    if let Some(_matches) = matches.subcommand_matches("admin") {
        // Check if admin exists, create otherwise
        create_admin_user(&pool).await?;
        return Ok(());
    }

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
                println!("Passwords does not match, retry.");
            }
        }
    } else {
        print!("{}", prompt);
        std::io::stdout().flush().unwrap();
        let mut value = String::new();
        std::io::stdin().read_line(&mut value).unwrap();
        value.trim().to_owned()
    }
}

async fn create_admin_user(pool: &Pool) -> ServiceResult<()> {
    let conn = &pool.get()?;

    let fullname = read_value("Fullname: ", false);
    let username = read_value("Username: ", false);
    let password = read_value("Password: ", true);

    let mut account = Account::create(&conn, &fullname, Permission::ADMIN)?;
    account.username = Some(username.clone());
    account.update(&conn)?;
    authentication_password::register(&conn, &account, &password)?;

    println!("Admin user '{}' was successfully created!", username);

    Ok(())
}
