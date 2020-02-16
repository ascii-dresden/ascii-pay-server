//! ---------------------------------------------------
//! |                                                 |
//! |             WARNING! - proxy demo               |
//! | Not for production! Only for frontend debugging |
//! |     Allows payments without authentication!     |
//! |      TODO: Remove proxy mod and references      |
//! |                                                 |
//! ---------------------------------------------------
pub mod sse;

use actix_web::{web, HttpResponse};
use std::sync::{Arc, Mutex};

use crate::api::transactions::Token;
use crate::core::{Account, Money, Pool, Product, ServiceResult};
use sse::Broadcaster;
use uuid::Uuid;

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", content = "content")]
#[serde(rename_all = "kebab-case")]
pub enum Message {
    Account {
        #[serde(flatten)]
        account: Account,
    },
    Product {
        #[serde(flatten)]
        product: Product,
    },
    QrCode {
        code: String,
    },
    NfcCard {
        id: String,
        name: String,
        writeable: bool,
    },
    RemoveNfcCard,
    PaymentToken {
        token: String,
    },
    Timeout,
}

pub fn setup() -> Arc<Mutex<Broadcaster>> {
    println!("---------------------------------------------------");
    println!("|                                                 |");
    println!("|             WARNING! - proxy demo               |");
    println!("| Not for production! Only for frontend debugging |");
    println!("|     Allows payments without authentication!     |");
    println!("|      TODO: Remove proxy mod and references      |");
    println!("|                                                 |");
    println!("---------------------------------------------------");
    let broadcaster = sse::Broadcaster::create();

    Broadcaster::spawn_ping(broadcaster.clone());

    broadcaster
}

pub fn init(config: &mut web::ServiceConfig) {
    config
        .service(web::resource("/events").to(sse::new_client))
        .service(
            web::resource("/request-payment-token").route(web::post().to(request_payment_token)),
        )
        .service(web::resource("/reauthenticate").route(web::get().to(request_reauthentication)))
        .service(
            web::scope("/proxy-demo")
                .service(web::resource("/account/{account_id}").route(web::get().to(get_account)))
                .service(web::resource("/product/{product_id}").route(web::get().to(get_product)))
                .service(web::resource("/qr-code/{code}").route(web::get().to(get_qr_code)))
                .service(
                    web::resource("/nfc-card/{id}/{name}/{writeable}")
                        .route(web::get().to(get_nfc_card)),
                )
                .service(
                    web::resource("/remove-nfc-card").route(web::get().to(get_remove_nfc_card)),
                )
                .service(
                    web::resource("/payment_token/{account_id}/{amount}")
                        .route(web::get().to(get_payment_token)),
                )
                .service(web::resource("/timeout").route(web::get().to(get_timeout))),
        );
}

#[derive(Debug, Deserialize)]
struct RequestPaymentToken {
    amount: i32,
}

async fn request_payment_token(
    _request: web::Json<RequestPaymentToken>,
) -> ServiceResult<HttpResponse> {
    Ok(HttpResponse::Ok().finish())
}

async fn request_reauthentication() -> ServiceResult<HttpResponse> {
    Ok(HttpResponse::Ok().finish())
}

async fn get_account(
    pool: web::Data<Pool>,
    params: web::Path<Uuid>,
    broadcaster: web::Data<Arc<Mutex<Broadcaster>>>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let account = Account::get(&conn, &params)?;

    let b = broadcaster.lock().unwrap();
    b.send(Message::Account { account });

    Ok(HttpResponse::Ok().finish())
}

async fn get_product(
    pool: web::Data<Pool>,
    params: web::Path<Uuid>,
    broadcaster: web::Data<Arc<Mutex<Broadcaster>>>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let product = Product::get(&conn, &params)?;

    let b = broadcaster.lock().unwrap();
    b.send(Message::Product { product });

    Ok(HttpResponse::Ok().finish())
}

async fn get_qr_code(
    params: web::Path<String>,
    broadcaster: web::Data<Arc<Mutex<Broadcaster>>>,
) -> ServiceResult<HttpResponse> {
    let b = broadcaster.lock().unwrap();
    b.send(Message::QrCode {
        code: params.into_inner(),
    });

    Ok(HttpResponse::Ok().finish())
}

async fn get_nfc_card(
    params: web::Path<(String, String, bool)>,
    broadcaster: web::Data<Arc<Mutex<Broadcaster>>>,
) -> ServiceResult<HttpResponse> {
    let (id, name, writeable) = params.into_inner();

    let b = broadcaster.lock().unwrap();
    b.send(Message::NfcCard {
        id,
        name,
        writeable,
    });

    Ok(HttpResponse::Ok().finish())
}
async fn get_remove_nfc_card(
    broadcaster: web::Data<Arc<Mutex<Broadcaster>>>,
) -> ServiceResult<HttpResponse> {
    let b = broadcaster.lock().unwrap();
    b.send(Message::RemoveNfcCard);

    Ok(HttpResponse::Ok().finish())
}

async fn get_payment_token(
    pool: web::Data<Pool>,
    params: web::Path<(Uuid, Money)>,
    broadcaster: web::Data<Arc<Mutex<Broadcaster>>>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;

    let (account_id, amount) = params.into_inner();

    let account = Account::get(&conn, &account_id)?;

    let b = broadcaster.lock().unwrap();
    b.send(Message::PaymentToken {
        token: Token::new(&conn, &account, amount)?.to_string()?,
    });

    Ok(HttpResponse::Ok().finish())
}

async fn get_timeout(
    broadcaster: web::Data<Arc<Mutex<Broadcaster>>>,
) -> ServiceResult<HttpResponse> {
    let b = broadcaster.lock().unwrap();
    b.send(Message::Timeout);

    Ok(HttpResponse::Ok().finish())
}
