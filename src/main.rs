#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate handlebars;
extern crate uuid;

use diesel::r2d2::{self, ConnectionManager};

mod api;
mod core;
mod server;
mod web;

use crate::core::{DbConnection, ServiceError};
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

    // Setup web server

    start_server(pool)?;

    Ok(())
}
