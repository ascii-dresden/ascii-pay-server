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
extern crate uuid;
#[macro_use]
extern crate hex_literal;
extern crate wallet_pass;
#[macro_use]
extern crate async_graphql;

use std::ops::Deref;

use clap::{App, SubCommand};
use diesel::PgConnection;
use log::{error, info};

// Internal services
mod demo_data;
mod grpc;
mod grpc_server;
mod http_server;
mod identity_service;
mod model;
mod repo;
mod utils;

// endpoints
mod api;

use crate::api::graphql::print_grahpql_schema;
use crate::demo_data::load_demo_data;
use crate::model::{authentication_password, Account, Product};
use crate::utils::{bb8_diesel, env, DatabasePool, ServiceResult};
use grpc_server::start_tcp_server;
use http_server::start_http_server;

embed_migrations!();

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    std::env::set_var(
        "RUST_LOG",
        "actix_web=info,actix_server=info,ascii_pay_server=info",
    );
    env_logger::init();

    let result = init().await;

    let exit_code = match result {
        Ok(_) => 0,
        Err(e) => {
            error!("{}", e);
            1
        }
    };

    std::process::exit(exit_code);
}

async fn init() -> ServiceResult<()> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .author(crate_authors!("\n"))
        .subcommand(SubCommand::with_name("run").about("Start the web server"))
        .subcommand(
            SubCommand::with_name("load-demo-data")
                .about("Initilize the database with demo data. This requires an empty database!"),
        )
        .subcommand(SubCommand::with_name("admin").about("Create a new admin user"))
        .subcommand(SubCommand::with_name("graphql").about("Print graphql definition"))
        .get_matches();

    if let Some(_matches) = matches.subcommand_matches("graphql") {
        print_grahpql_schema();
        return Ok(());
    }

    // Setup database connection
    let database_manager =
        bb8_diesel::DieselConnectionManager::<PgConnection>::new(env::DATABASE_URI.as_str());
    let database_pool = bb8::Pool::builder().build(database_manager).await?;

    Product::load_dataset()?;

    // Setup redis connection
    let redis_manager = bb8_redis::RedisConnectionManager::new(env::REDIS_URI.as_str()).unwrap();
    let redis_pool = bb8::Pool::builder().build(redis_manager).await?;

    {
        let conn = database_pool.get().await?;
        embedded_migrations::run_with_output(conn.deref(), &mut std::io::stdout())?;
    }

    if let Some(_matches) = matches.subcommand_matches("run") {
        // Setup web server
        start_tcp_server(database_pool.clone(), redis_pool.clone());
        start_http_server(database_pool, redis_pool).await?;
        return Ok(());
    }

    if let Some(_matches) = matches.subcommand_matches("admin") {
        // Check if admin exists, create otherwise
        create_admin_user(&database_pool).await?;
        return Ok(());
    }

    if let Some(_matches) = matches.subcommand_matches("load-demo-data") {
        load_demo_data(&database_pool).await?;
        return Ok(());
    }

    Ok(())
}

async fn create_admin_user(database_pool: &DatabasePool) -> ServiceResult<()> {
    let mut password = String::new();
    std::io::stdin().read_line(&mut password)?;
    password = password.trim().to_owned();

    let (account, created) =
        Account::create_admin_account(database_pool, "Administrator", "admin").await?;
    authentication_password::register(database_pool, &account, &password).await?;

    if created {
        info!("Admin user was successfully created!");
    } else {
        info!("Admin user was successfully updated!");
    }

    Ok(())
}
