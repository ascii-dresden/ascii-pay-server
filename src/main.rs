#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate block_modes;
extern crate clap;
extern crate handlebars;
extern crate uuid;
#[macro_use]
extern crate hex_literal;

use clap::App;
use diesel::r2d2::{self, ConnectionManager};

mod api;
mod core;
mod identity_policy;
mod init_server;
mod server;
mod web;

use crate::core::{authentication_password, env, Account, DbConnection, Pool, ServiceResult};
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
    check_admin_user_exisits(&pool).await?;

    // Setup web server
    start_server(pool).await?;

    Ok(())
}

/// Check if a initial user exists. Otherwise create a new one
async fn check_admin_user_exisits(pool: &Pool) -> ServiceResult<()> {
    let conn = &pool.get().unwrap();

    let admin_with_password_exists = Account::all(&conn)?
        .iter()
        .filter(|a| a.permission.is_admin())
        .any(|a| authentication_password::has_password(&conn, a).unwrap_or(false));

    if !admin_with_password_exists {
        println!("You seem to have started the server on an empty database. We'll now create the initial superuser.");
        init_server::start_server(pool).await?;

        println!("Init server finished, continue to normal server");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}
