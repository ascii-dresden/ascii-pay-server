use crate::client_cert_required;
use crate::core::authentication_nfc::NfcResult;
use crate::core::{
    authentication_barcode, authentication_nfc, wallet, Account, Pool, Product, ServiceResult,
};
use crate::identity_policy::Action;

use actix_web::{web, HttpRequest, HttpResponse};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum IdentificationRequest {
    Barcode {
        code: String,
    },
    Nfc {
        id: String,
    },
    NfcSecret {
        id: String,
        challenge: String,
        response: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum IdentificationResponse {
    Account {
        account: Account,
    },
    Product {
        product: Product,
    },
    AuthenticationNeeded {
        id: String,
        key: String,
        challenge: String,
    },
    WriteKey {
        id: String,
        key: String,
        secret: String,
    },
}

/// POST route for `/api/v1/identify`
pub async fn post_identify(
    pool: web::Data<Pool>,
    identification_request: web::Json<IdentificationRequest>,
    request: HttpRequest,
) -> ServiceResult<HttpResponse> {
    client_cert_required!(request, Action::FORBIDDEN);

    let conn = &pool.get()?;
    let identification_request = identification_request.into_inner();

    match identification_request {
        IdentificationRequest::Barcode { code } => {
            if let Ok(product) = Product::get_by_barcode(&conn, &code) {
                return Ok(HttpResponse::Ok().json(&IdentificationResponse::Product { product }));
            }

            if let Ok(account_id) = wallet::get_by_qr_code(&conn, &code) {
                let account = Account::get(conn, &account_id)?;
                return Ok(HttpResponse::Ok().json(&IdentificationResponse::Account { account }));
            }

            let account = authentication_barcode::get(&conn, &code)?;
            Ok(HttpResponse::Ok().json(&IdentificationResponse::Account { account }))
        }
        IdentificationRequest::Nfc { id } => match authentication_nfc::get(&conn, &id)? {
            NfcResult::Ok { account } => {
                Ok(HttpResponse::Ok().json(&IdentificationResponse::Account { account }))
            }
            NfcResult::AuthenticationRequested { key, challenge } => Ok(HttpResponse::Ok()
                .json(&IdentificationResponse::AuthenticationNeeded { id, key, challenge })),
            NfcResult::WriteKey { key, secret } => {
                Ok(HttpResponse::Ok().json(&IdentificationResponse::WriteKey { id, key, secret }))
            }
        },
        IdentificationRequest::NfcSecret {
            id,
            challenge,
            response,
        } => {
            let account =
                authentication_nfc::get_challenge_response(&conn, &id, &challenge, &response)?;
            Ok(HttpResponse::Ok().json(&IdentificationResponse::Account { account }))
        }
    }
}
