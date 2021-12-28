use uuid::Uuid;

use crate::identity_service::{Identity, IdentityRequire};
use crate::model::session::create_onetime_session;
use crate::model::{authentication_nfc, redis, wallet, Account, Permission, Product};
use crate::utils::{
    bytes_to_string, generate_key_array, mifare_utils, str_to_bytes, vec_to_array, DatabasePool,
    RedisPool, ServiceError, ServiceResult,
};

#[derive(Debug, Clone, Copy)]
pub enum TokenType {
    AccountAccessToken,
    ProductId,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Enum)]
pub enum NfcCardType {
    Generic,
    MifareDesfire,
}

const CARD_TYPE_GENERIC: &str = "generic";
const CARD_TYPE_MIFARE_DESFIRE: &str = "mifare-desfire";

pub type Token = (TokenType, String);

#[derive(Debug, Serialize, SimpleObject)]
pub struct AccountAccessTokenOutput {
    pub token: String,
}

pub async fn authenticate_account(
    database_pool: &DatabasePool,
    redis_pool: &RedisPool,
    identity: &Identity,
    account_id: Uuid,
) -> ServiceResult<AccountAccessTokenOutput> {
    identity.require_account(Permission::Admin)?;

    let account = Account::get(database_pool, account_id).await?;
    let session = create_onetime_session(redis_pool, &account).await?;

    Ok(AccountAccessTokenOutput {
        token: session.into(),
    })
}

pub async fn authenticate_barcode(
    database_pool: &DatabasePool,
    redis_pool: &RedisPool,
    identity: &Identity,
    code: &str,
) -> ServiceResult<Token> {
    identity.require_cert()?;

    if let Ok(product) = Product::get_by_barcode(code) {
        return Ok((TokenType::ProductId, product.id));
    }

    if let Ok(account_id) = wallet::get_by_qr_code(database_pool, code).await {
        let account = Account::get(database_pool, account_id).await?;
        let session = create_onetime_session(redis_pool, &account).await?;

        return Ok((TokenType::AccountAccessToken, session.into()));
    }

    Err(ServiceError::NotFound)
}

pub async fn authenticate_nfc_type(
    database_pool: &DatabasePool,
    identity: &Identity,
    id: &str,
) -> ServiceResult<NfcCardType> {
    identity.require_cert()?;

    let nfc_entry = authentication_nfc::get_by_card_id(database_pool, id).await?;

    match nfc_entry.card_type.as_str() {
        CARD_TYPE_GENERIC => Ok(NfcCardType::Generic),
        CARD_TYPE_MIFARE_DESFIRE => Ok(NfcCardType::MifareDesfire),
        _ => Err(ServiceError::InternalServerError(
            "Unsupported card type!",
            nfc_entry.card_type,
        )),
    }
}

pub async fn authenticate_nfc_generic(
    database_pool: &DatabasePool,
    redis_pool: &RedisPool,
    identity: &Identity,
    id: &str,
) -> ServiceResult<Token> {
    identity.require_cert()?;

    let nfc_entry = authentication_nfc::get_by_card_id(database_pool, id).await?;
    if nfc_entry.card_type != CARD_TYPE_GENERIC {
        return Err(ServiceError::Unauthorized("nfc card type does not match!"));
    }

    let account = Account::get(database_pool, nfc_entry.account_id).await?;
    let session = create_onetime_session(redis_pool, &account).await?;

    Ok((TokenType::AccountAccessToken, session.into()))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct AuthenticateNfcMifareDesfire {
    rnd_a: [u8; 8],
    rnd_b: [u8; 8],
}

#[allow(non_snake_case)]
pub async fn authenticate_nfc_mifare_desfire_phase1(
    database_pool: &DatabasePool,
    redis_pool: &RedisPool,
    identity: &Identity,
    id: &str,
    ek_rndB: &str,
) -> ServiceResult<String> {
    identity.require_cert()?;

    let nfc_entry = authentication_nfc::get_by_card_id(database_pool, id).await?;
    if nfc_entry.card_type != CARD_TYPE_MIFARE_DESFIRE {
        return Err(ServiceError::Unauthorized("nfc card type does not match!"));
    }

    let key = str_to_bytes(&nfc_entry.data);
    let ek_rndB = str_to_bytes(ek_rndB);

    let rndA = mifare_utils::generate_key();
    let rndB = vec_to_array(mifare_utils::tdes_decrypt(&key, &ek_rndB)?)?;

    let mut rndBshifted: Vec<u8> = Vec::with_capacity(8);
    rndBshifted.extend(&rndB[1..8]);
    rndBshifted.push(rndB[0]);

    let mut rndA_rndBshifted: Vec<u8> = Vec::with_capacity(16);
    rndA_rndBshifted.extend(&rndA);
    rndA_rndBshifted.extend(rndBshifted);

    let dk_rndA_rndBshifted = mifare_utils::tdes_encrypt(&key, &rndA_rndBshifted)?;

    redis::create_data::<AuthenticateNfcMifareDesfire>(
        redis_pool,
        id,
        &AuthenticateNfcMifareDesfire {
            rnd_a: rndA,
            rnd_b: rndB,
        },
        10,
    )
    .await?;

    Ok(bytes_to_string(&dk_rndA_rndBshifted))
}

#[allow(non_snake_case)]
pub async fn authenticate_nfc_mifare_desfire_phase2(
    database_pool: &DatabasePool,
    redis_pool: &RedisPool,
    identity: &Identity,
    id: &str,
    dk_rndA_rndBshifted: &str,
    ek_rndAshifted_card: &str,
) -> ServiceResult<(String, Token)> {
    identity.require_cert()?;

    let nfc_entry = authentication_nfc::get_by_card_id(database_pool, id).await?;
    if nfc_entry.card_type != CARD_TYPE_MIFARE_DESFIRE {
        return Err(ServiceError::Unauthorized("nfc card type does not match!"));
    }

    let key = str_to_bytes(&nfc_entry.data);
    let redis_cache =
        redis::get_delete_data::<AuthenticateNfcMifareDesfire>(redis_pool, id).await?;
    let rndA = redis_cache.rnd_a;
    let rndB = redis_cache.rnd_b;

    let mut rndBshifted: Vec<u8> = Vec::with_capacity(8);
    rndBshifted.extend(&rndB[1..8]);
    rndBshifted.push(rndB[0]);

    let mut rndA_rndBshifted: Vec<u8> = Vec::with_capacity(16);
    rndA_rndBshifted.extend(&rndA);
    rndA_rndBshifted.extend(rndBshifted);

    let dk_rndA_rndBshifted_ref = mifare_utils::tdes_encrypt(&key, &rndA_rndBshifted)?;
    if dk_rndA_rndBshifted != bytes_to_string(&dk_rndA_rndBshifted_ref) {
        return Err(ServiceError::Unauthorized(
            "response does not match challenge!",
        ));
    }

    let ek_rndAshifted_card = str_to_bytes(ek_rndAshifted_card);
    let rndAshifted_card = mifare_utils::tdes_decrypt(&key, &ek_rndAshifted_card)?;

    let mut rndAshifted: Vec<u8> = Vec::with_capacity(8);
    rndAshifted.extend(&rndA[1..8]);
    rndAshifted.push(rndA[0]);

    if rndAshifted != rndAshifted_card {
        return Err(ServiceError::Unauthorized("challenge response failed!"));
    }

    let mut session_key: Vec<u8> = Vec::with_capacity(16);
    session_key.extend(&rndA[0..4]);
    session_key.extend(&rndB[0..4]);
    if mifare_utils::is_key_2des(&key) {
        session_key.extend(&rndA[4..8]);
        session_key.extend(&rndB[4..8]);
    }

    let account = Account::get(database_pool, nfc_entry.account_id).await?;
    let session = create_onetime_session(redis_pool, &account).await?;

    Ok((
        bytes_to_string(&session_key),
        (TokenType::AccountAccessToken, session.into()),
    ))
}

pub async fn authenticate_nfc_delete_card(
    database_pool: &DatabasePool,
    identity: &Identity,
    account_id: Uuid,
    card_id: &str,
) -> ServiceResult<()> {
    identity.require_cert()?;

    let account = Account::get(database_pool, account_id).await?;

    authentication_nfc::remove(database_pool, &account, card_id).await
}

pub async fn authenticate_nfc_generic_init_card(
    database_pool: &DatabasePool,
    identity: &Identity,
    card_id: &str,
    account_id: Uuid,
) -> ServiceResult<()> {
    identity.require_cert()?;

    let account = Account::get(database_pool, account_id).await?;

    authentication_nfc::register(
        database_pool,
        &account,
        card_id,
        CARD_TYPE_GENERIC,
        "Generic NFC Card",
        "",
    )
    .await
}

pub async fn authenticate_nfc_mifare_desfire_init_card(
    database_pool: &DatabasePool,
    identity: &Identity,
    card_id: &str,
    account_id: Uuid,
) -> ServiceResult<String> {
    identity.require_cert()?;

    let account = Account::get(database_pool, account_id).await?;
    let key = bytes_to_string(&generate_key_array::<16>());

    authentication_nfc::register(
        database_pool,
        &account,
        card_id,
        CARD_TYPE_MIFARE_DESFIRE,
        "Ascii Pay Card",
        &key,
    )
    .await?;

    Ok(key)
}
