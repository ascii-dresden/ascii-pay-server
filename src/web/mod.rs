pub mod admin;
pub mod default;
pub mod login;
pub mod utils;

// TODO: REMOVE FOR PRODUCTION!
pub mod proxy;

use crate::core::ServiceResult;
use crate::web::utils::HbData;
use actix_files as fs;
use actix_web::{web, HttpRequest, HttpResponse};
use handlebars::Handlebars;

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
            .configure(default::init)
            .default_service(web::get().to(get_404)),
    );
}

/// GET route for 404 error.
pub async fn get_404(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let body = HbData::new(&request).render(&hb, "404")?;

    Ok(HttpResponse::Ok().body(body))
}
