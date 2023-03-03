use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;

use block_modes::block_padding::ZeroPadding;
use block_modes::cipher::{BlockCipher, BlockDecrypt, BlockEncrypt, NewBlockCipher};
use block_modes::{BlockMode, Cbc};
use chrono::{Duration, Utc};
use des::TdesEde2;
use generic_array::GenericArray;
use rand::RngCore;
use tokio::sync::Mutex;

use crate::database::AppStateNfcChallenge;
use crate::error::{ServiceError, ServiceResult};
use crate::models::AuthNfc;

/// Communication to the mifare desfire always requires the tdes decribt
struct MiFareTdes {
    cipher: TdesEde2,
}

impl NewBlockCipher for MiFareTdes {
    type KeySize = <TdesEde2 as NewBlockCipher>::KeySize;

    fn new(key: &GenericArray<u8, Self::KeySize>) -> Self {
        MiFareTdes {
            cipher: TdesEde2::new(key),
        }
    }
}

impl BlockCipher for MiFareTdes {
    type BlockSize = <TdesEde2 as BlockCipher>::BlockSize;
    type ParBlocks = <TdesEde2 as BlockCipher>::ParBlocks;
}

impl BlockEncrypt for MiFareTdes {
    fn encrypt_block(&self, block: &mut GenericArray<u8, Self::BlockSize>) {
        self.cipher.decrypt_block(block)
    }
}

impl BlockDecrypt for MiFareTdes {
    fn decrypt_block(&self, block: &mut GenericArray<u8, Self::BlockSize>) {
        self.cipher.decrypt_block(block)
    }
}

fn tdes_encrypt(key: &[u8], value: &[u8]) -> ServiceResult<Vec<u8>> {
    let mut v = Vec::with_capacity(16);
    v.extend(key);

    if key.len() == 8 {
        v.extend(key);
    }

    let key = GenericArray::from_slice(&v);

    let iv = GenericArray::from_slice(&[0u8; 8]);
    let cipher: Cbc<MiFareTdes, ZeroPadding> = Cbc::new(MiFareTdes::new(key), iv);

    Ok(cipher.encrypt_vec(value))
}

fn tdes_decrypt(key: &[u8], value: &[u8]) -> ServiceResult<Vec<u8>> {
    let mut v = Vec::with_capacity(16);
    v.extend(key);

    if key.len() == 8 {
        v.extend(key);
    }

    let key = GenericArray::from_slice(&v);

    let iv = GenericArray::from_slice(&[0u8; 8]);
    let cipher: Cbc<MiFareTdes, ZeroPadding> = Cbc::new(MiFareTdes::new(key), iv);

    Ok(cipher.decrypt_vec(value)?)
}

fn is_key_2des(key: &[u8]) -> bool {
    if key.len() == 8 {
        return false;
    }

    if key.len() == 16 && key[0..8] == key[8..16] {
        return false;
    }

    true
}

fn generate_key() -> [u8; 8] {
    let mut data = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut data);
    data
}

fn vec_to_array<T, const N: usize>(v: Vec<T>) -> ServiceResult<[T; N]> {
    let len = v.len();
    let r: Result<[T; N], _> = v.try_into();
    r.map_err(|_| {
        ServiceError::InternalServerError(format!("Expected a Vec of length {N} but it was {len}"))
    })
}

#[allow(non_snake_case)]
pub async fn authenticate_phase_challenge(
    challenge_storage: &Arc<Mutex<HashMap<u64, AppStateNfcChallenge>>>,
    account_id: u64,
    auth_nfc: &AuthNfc,
    request: &[u8],
) -> ServiceResult<Vec<u8>> {
    let ek_rndB = request;
    let key = auth_nfc.data.clone();

    let rndA = generate_key();
    let rndB = vec_to_array::<u8, 8>(tdes_decrypt(&key, ek_rndB)?)?;
    let state = AppStateNfcChallenge {
        valid_until: Utc::now().add(Duration::seconds(10)),
        rnd_a: rndA.to_vec(),
        rnd_b: rndB.to_vec(),
    };

    let mut rndBshifted: Vec<u8> = Vec::with_capacity(8);
    rndBshifted.extend(&rndB[1..8]);
    rndBshifted.push(rndB[0]);

    let mut rndA_rndBshifted: Vec<u8> = Vec::with_capacity(16);
    rndA_rndBshifted.extend(rndA);
    rndA_rndBshifted.extend(rndBshifted);

    let dk_rndA_rndBshifted = tdes_encrypt(&key, &rndA_rndBshifted)?;

    let mut map = challenge_storage.lock().await;
    map.insert(account_id, state);

    Ok(dk_rndA_rndBshifted)
}

#[allow(non_snake_case)]
pub async fn authenticate_phase_response(
    challenge_storage: &Arc<Mutex<HashMap<u64, AppStateNfcChallenge>>>,
    account_id: u64,
    auth_nfc: &AuthNfc,
    challenge: &[u8],
    response: &[u8],
) -> ServiceResult<Vec<u8>> {
    let dk_rndA_rndBshifted = challenge;
    let ek_rndAshifted_card = response;
    let key = auth_nfc.data.clone();

    let mut map = challenge_storage.lock().await;
    let state = map.remove(&account_id).ok_or(ServiceError::Unauthorized(
        "response does not match challenge!",
    ))?;

    if state.valid_until < Utc::now() {
        return Err(ServiceError::Unauthorized("challenge response timeout!"));
    }

    let rndA = state.rnd_a;
    let rndB = state.rnd_b;

    let mut rndBshifted: Vec<u8> = Vec::with_capacity(8);
    rndBshifted.extend(&rndB[1..8]);
    rndBshifted.push(rndB[0]);

    let mut rndA_rndBshifted: Vec<u8> = Vec::with_capacity(16);
    rndA_rndBshifted.extend(&rndA);
    rndA_rndBshifted.extend(rndBshifted);

    let dk_rndA_rndBshifted_ref = tdes_encrypt(&key, &rndA_rndBshifted)?;
    if dk_rndA_rndBshifted != dk_rndA_rndBshifted_ref {
        return Err(ServiceError::Unauthorized(
            "response does not match challenge!",
        ));
    }

    let rndAshifted_card = tdes_decrypt(&key, ek_rndAshifted_card)?;

    let mut rndAshifted: Vec<u8> = Vec::with_capacity(8);
    rndAshifted.extend(&rndA[1..8]);
    rndAshifted.push(rndA[0]);

    if rndAshifted != rndAshifted_card {
        return Err(ServiceError::Unauthorized("challenge response failed!"));
    }

    let mut session_key: Vec<u8> = Vec::with_capacity(16);
    session_key.extend(&rndA[0..4]);
    session_key.extend(&rndB[0..4]);
    if is_key_2des(&key) {
        session_key.extend(&rndA[4..8]);
        session_key.extend(&rndB[4..8]);
    }

    Ok(session_key)
}
