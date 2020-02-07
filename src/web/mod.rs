pub mod admin;
pub mod default;
pub mod login;
pub mod utils;

use actix_files as fs;
use actix_web::web;

/// Setup routes for admin ui
pub fn init(config: &mut web::ServiceConfig) {
    admin::init(config);
    
    config.service(
        web::scope("/")
            // Setup static routes
            .service(fs::Files::new("/stylesheets", "static/stylesheets/"))
            .service(fs::Files::new("/javascripts", "static/javascripts/"))
            .service(fs::Files::new("/images", "static/images/"))
            .service(fs::Files::new("/product/image", "img/"))
            // Setup login routes
            .service(
                web::resource("/login")
                    .route(web::post().to(login::post_login))
                    .route(web::get().to(login::get_login)),
            )
            .service(web::resource("/logout").route(web::get().to(login::get_logout)))
            .service(
                web::resource("/register/{invitation_id}")
                    .route(web::post().to(login::post_register))
                    .route(web::get().to(login::get_register)),
            )
            .configure(default::init),
    );
}
