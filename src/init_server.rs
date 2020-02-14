use actix_files as fs;
use actix_rt::System;
use actix_web::dev::Server;
use actix_web::{http, HttpRequest, HttpResponse};
use actix_web::{middleware, web, App, HttpServer};
use handlebars::Handlebars;
use std::sync::{Arc, Mutex};

use crate::core::{authentication_password, env, Account, Permission, Pool, ServiceResult};
use crate::web::utils::HbData;

pub struct ServerHandler {
    pub server: Option<Server>,
}

/// Start a new actix server with the given database pool
pub async fn start_server(pool: &Pool) -> ServiceResult<()> {
    // Read config params from env

    let address = format!("{}:{}", env::HOST.as_str(), *env::PORT);

    let mut handlebars = Handlebars::new();

    // Set handlebars template directory
    handlebars
        .register_templates_directory(".handlebars", "./static/templates")
        .unwrap();

    // Move handlebars reference to actix
    let handlebars_ref = web::Data::new(handlebars);

    let cloned_pool = pool.clone();

    std::thread::spawn(move || {
        let sys = System::new("http-server");

        let handler = Arc::new(Mutex::new(ServerHandler { server: None }));

        let handler_copy = handler.clone();

        let srv = HttpServer::new(move || {
            App::new()
                // Move database pool
                .data(cloned_pool.clone())
                // Set handlebars reference
                .app_data(handlebars_ref.clone())
                .data(handler_copy.clone())
                // Logger
                .wrap(middleware::Logger::default())
                // Register api module
                .service(
                    web::scope("/")
                        // Setup static routes
                        .service(fs::Files::new("/stylesheets", "static/stylesheets/"))
                        .service(fs::Files::new("/images", "static/images/"))
                        // Setup login routes
                        .service(
                            web::resource("")
                                .route(web::post().to(post_init))
                                .route(web::get().to(get_init)),
                        )
                        .default_service(web::get().to(get_404)),
                )
        })
        .bind(address)
        .unwrap()
        .run();

        {
            let mut h = handler.lock().unwrap();
            h.server = Some(srv);
        }

        sys.run().unwrap();
    })
    .join()
    .unwrap();

    Ok(())
}

/// GET route for 404 error.
async fn get_404() -> ServiceResult<HttpResponse> {
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish())
}

#[derive(Debug, Serialize, Deserialize)]
struct InitForm {
    fullname: String,
    username: String,
    password: String,
    password2: String,
}

/// GET route for `/` if user is not logged in
async fn get_init(
    hb: web::Data<Handlebars<'_>>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    let body = HbData::new(&request)
        .with_data("error", &request.query_string().contains("error"))
        .render(&hb, "init")?;

    Ok(HttpResponse::Ok().body(body))
}

/// POST route for `/`
async fn post_init(
    pool: web::Data<Pool>,
    params: web::Form<InitForm>,
    handler: web::Data<Arc<Mutex<ServerHandler>>>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    if params.password.is_empty()
        || params.password != params.password2
        || params.username.is_empty()
    {
        return Ok(HttpResponse::Found()
            .header(http::header::LOCATION, "/?error")
            .finish());
    }

    let mut account = Account::create(&conn, &params.fullname, Permission::ADMIN)?;
    account.username = Some(params.username.clone());
    account.update(&conn)?;
    authentication_password::register(&conn, &account, &params.password)?;

    if let Some(svr) = &handler.lock().unwrap().server {
        svr.stop(false).await;
    }
    actix_rt::System::current().stop();

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish())
}
