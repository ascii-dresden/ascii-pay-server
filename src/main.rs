#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
extern crate uuid;

use diesel::r2d2::{self, ConnectionManager};

mod core;
mod web;

use crate::core::{DbConnection, ServiceError};

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

    // Setup web server
    let domain = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());
    let host = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "".to_string())
        .parse::<i32>()
        .unwrap_or(8080);

    web::init(&domain, &host, port, pool)?;

    Ok(())
}
