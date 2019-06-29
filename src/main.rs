#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
extern crate rpassword;

use std::io::{stdin, Write};
use std::io::stdout;

use actix::prelude::*;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{App, HttpServer, web};
use actix_web::middleware::Logger;
use chrono::{Duration, Utc};
use diesel::{r2d2::ConnectionManager, SqliteConnection};
use uuid::Uuid;

use dotenv::dotenv;

use crate::errors::ServiceError;
use crate::models::{Account, DbExecutor, User};
use crate::utils::hash_password;

mod schema;
mod errors;
mod models;
mod utils;

mod auth_handler;
mod auth_routes;

fn read_value(prompt: &str, hide_input: bool) -> String {
    if hide_input {
        return rpassword::prompt_password_stdout(prompt).unwrap()
    } else {
        print!("{}", prompt);
        stdout().flush().unwrap();
        let mut value = String::new();
        stdin().read_line(&mut value).unwrap();
        return value.trim().to_owned();
    }
}

fn check_fallback_user(conn: &SqliteConnection) -> Result<(), ServiceError> {
    use diesel::prelude::*;
    use crate::schema::users::dsl::users;
    use crate::schema::accounts::dsl::accounts;

    let items = users.limit(1).load::<User>(conn)?;

    if items.is_empty() {
        println!("Create the admin user:");
        let user = read_value("username: ", false);
        let mail = read_value("mail:     ", false);
        let pass = read_value("password: ", true);

        println!("Create user '{}'({}) with pw: '{}'", &user, &mail, "*".repeat(pass.len()));

        let account = Account {
            id: format!("{}", Uuid::new_v4().hyphenated()),
            display: user,
            credit: 0,
            limit: 0,
            created: Utc::now().naive_utc(),
            updated: Utc::now().naive_utc(),
        };

        let user = User {
            id: format!("{}", Uuid::new_v4().hyphenated()),
            account_id: account.id.to_string(),
            first_name: "".to_owned(),
            last_name: "".to_owned(),
            mail,
            password: hash_password(&pass).unwrap(),
            created: Utc::now().naive_utc(),
            updated: Utc::now().naive_utc(),
        };

        diesel::insert_into(accounts).values(&account).execute(conn).unwrap();
        diesel::insert_into(users).values(&user).execute(conn).unwrap();
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();
    let sys = actix_rt::System::new("ascii-prepaid-system");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // create db connection pool
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let conn: &SqliteConnection = &pool.get().unwrap();
    check_fallback_user(conn).unwrap();

    let addr_db_executor: Addr<DbExecutor> =
        SyncArbiter::start(4, move || DbExecutor(pool.clone()));

    let domain = std::env::var("DOMAIN")
        .unwrap_or_else(|_| "localhost".to_string());
    let host = std::env::var("HOST")
        .unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "".to_string())
        .parse::<i32>().unwrap_or(8080);

    let address = format!("{}:{}", &host, port);

    HttpServer::new(move || {
        // secret is a random minimum 32 bytes long base 64 string
        let secret: String =
            std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(8));

        App::new()
            .data(addr_db_executor.clone())
            .wrap(Logger::default())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(secret.as_bytes())
                    .name("auth")
                    .path("/")
                    .domain(domain.as_str())
                    .max_age_time(Duration::days(1))
                    .secure(false), // this can only be true if you have https
            ))
            // everything under '/api/' route
            .service(
                web::scope("/api")
                    // routes for authentication
                    .service(
                        web::resource("/auth")
                            .route(web::post().to_async(auth_routes::login))
                            .route(web::delete().to(auth_routes::logout))
                            .route(web::get().to_async(auth_routes::get_me)),
                    )
            )
    })
        .bind(&address)?
        .start();

    println!("Listen to http://{}", &address);

    sys.run()
}
