use crate::identity_service::{Identity, IdentityRequire};
use crate::model::authentication_nfc::NfcResult;
use crate::model::{
    authentication_barcode, authentication_nfc, wallet, Account, DbConnection, Product,
    ServiceResult,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum IdentificationInput {
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
pub enum IdentificationOutput {
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

pub fn identify(
    conn: &DbConnection,
    identity: &Identity,
    input: IdentificationInput,
) -> ServiceResult<IdentificationOutput> {
    identity.require_cert()?;

    match input {
        IdentificationInput::Barcode { code } => {
            if let Ok(product) = Product::get_by_barcode(conn, &code) {
                return Ok(IdentificationOutput::Product { product });
            }

            if let Ok(account_id) = wallet::get_by_qr_code(conn, &code) {
                let account = Account::get(conn, &account_id)?;
                return Ok(IdentificationOutput::Account { account });
            }

            let account = authentication_barcode::get(conn, &code)?;
            Ok(IdentificationOutput::Account { account })
        }
        IdentificationInput::Nfc { id } => match authentication_nfc::get(conn, &id)? {
            NfcResult::Ok { account } => Ok(IdentificationOutput::Account { account }),
            NfcResult::AuthenticationRequested { key, challenge } => {
                Ok(IdentificationOutput::AuthenticationNeeded { id, key, challenge })
            }
            NfcResult::WriteKey { key, secret } => {
                Ok(IdentificationOutput::WriteKey { id, key, secret })
            }
        },
        IdentificationInput::NfcSecret {
            id,
            challenge,
            response,
        } => {
            let account =
                authentication_nfc::get_challenge_response(conn, &id, &challenge, &response)?;
            Ok(IdentificationOutput::Account { account })
        }
    }
}
