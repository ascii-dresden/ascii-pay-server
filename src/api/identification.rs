use crate::core::authentication_nfc::NfcResult;
use crate::core::{
    authentication_barcode, authentication_nfc, Account, Pool, Product, ServiceResult,
};
use actix_web::{web, HttpResponse};

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
        #[serde(flatten)]
        account: Account,
    },
    Product {
        #[serde(flatten)]
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
    request: web::Json<IdentificationRequest>,
) -> ServiceResult<HttpResponse> {
    let conn = &pool.get()?;
    let request = request.into_inner();

    match request {
        IdentificationRequest::Barcode { code } => {
            if let Ok(product) = Product::get_by_barcode(&conn, &code) {
                return Ok(HttpResponse::Ok().json(&IdentificationResponse::Product { product }));
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
            NfcResult::WriteKey { key, secret } =>  Ok(HttpResponse::Ok()
                .json(&IdentificationResponse::WriteKey { id, key, secret })),
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
