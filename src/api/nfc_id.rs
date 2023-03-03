use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;

use aes::Aes256;
use block_modes::block_padding::ZeroPadding;
use block_modes::cipher::{BlockCipher, BlockDecrypt, BlockEncrypt, NewBlockCipher};
use block_modes::{BlockMode, Cbc};
use chrono::{Duration, Utc};
use generic_array::GenericArray;
use hex_literal::hex;
use rand::RngCore;
use tokio::sync::Mutex;

use crate::database::AppStateNfcChallenge;
use crate::error::{ServiceError, ServiceResult};
use crate::models::{AuthNfc, CardType};

/// Communication to the mifare desfire always requires the tdes decribt
struct NfcAes {
    cipher: Aes256,
}

impl NewBlockCipher for NfcAes {
    type KeySize = <Aes256 as NewBlockCipher>::KeySize;

    fn new(key: &GenericArray<u8, Self::KeySize>) -> Self {
        NfcAes {
            cipher: Aes256::new(key),
        }
    }
}

impl BlockCipher for NfcAes {
    type BlockSize = <Aes256 as BlockCipher>::BlockSize;
    type ParBlocks = <Aes256 as BlockCipher>::ParBlocks;
}

impl BlockEncrypt for NfcAes {
    fn encrypt_block(&self, block: &mut GenericArray<u8, Self::BlockSize>) {
        self.cipher.encrypt_block(block)
    }
}

impl BlockDecrypt for NfcAes {
    fn decrypt_block(&self, block: &mut GenericArray<u8, Self::BlockSize>) {
        self.cipher.decrypt_block(block)
    }
}

fn aes_encrypt(key: &[u8], value: &[u8]) -> ServiceResult<Vec<u8>> {
    let key = GenericArray::from_slice(key);

    let iv = GenericArray::from_slice(&[0u8; 32]);
    let cipher: Cbc<NfcAes, ZeroPadding> = Cbc::new(NfcAes::new(key), iv);

    Ok(cipher.encrypt_vec(value))
}

fn aes_decrypt(key: &[u8], value: &[u8]) -> ServiceResult<Vec<u8>> {
    let key = GenericArray::from_slice(key);

    let iv = GenericArray::from_slice(&[0u8; 32]);
    let cipher: Cbc<NfcAes, ZeroPadding> = Cbc::new(NfcAes::new(key), iv);

    Ok(cipher.decrypt_vec(value)?)
}

fn generate_key() -> [u8; 32] {
    let mut data = [0u8; 32];
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

fn get_reader_key() -> Vec<u8> {
    let key = hex!("c50ab42b5d32b6ccf26b4c5d2e9862c7694cfba9a8eac568a36e1f400a0f480d");
    Vec::from(key)
}

#[allow(non_snake_case)]
pub async fn authenticate_phase_challenge(
    challenge_storage: &Arc<Mutex<HashMap<u64, AppStateNfcChallenge>>>,
    account_id: u64,
    auth_nfc: &AuthNfc,
    request: &[u8],
) -> ServiceResult<Vec<u8>> {
    let ek_rndB = request;
    let key = match auth_nfc.card_type {
        CardType::GenericNfc => get_reader_key(),
        _ => auth_nfc.data.clone(),
    };

    let rndA = generate_key();
    let rndB = vec_to_array::<u8, 32>(aes_decrypt(&key, ek_rndB)?)?;
    let state = AppStateNfcChallenge {
        valid_until: Utc::now().add(Duration::seconds(10)),
        rnd_a: rndA.to_vec(),
        rnd_b: rndB.to_vec(),
    };

    let mut rndBshifted: Vec<u8> = Vec::with_capacity(32);
    rndBshifted.extend(&rndB[1..32]);
    rndBshifted.push(rndB[0]);

    let mut rndA_rndBshifted: Vec<u8> = Vec::with_capacity(64);
    rndA_rndBshifted.extend(rndA);
    rndA_rndBshifted.extend(rndBshifted);

    let dk_rndA_rndBshifted = aes_encrypt(&key, &rndA_rndBshifted)?;

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
    let key = match auth_nfc.card_type {
        CardType::GenericNfc => get_reader_key(),
        _ => auth_nfc.data.clone(),
    };

    let mut map = challenge_storage.lock().await;
    let state = map.remove(&account_id).ok_or(ServiceError::Unauthorized(
        "response does not match challenge!",
    ))?;

    if state.valid_until < Utc::now() {
        return Err(ServiceError::Unauthorized("challenge response timeout!"));
    }

    let rndA = state.rnd_a;
    let rndB = state.rnd_b;

    let mut rndBshifted: Vec<u8> = Vec::with_capacity(32);
    rndBshifted.extend(&rndB[1..32]);
    rndBshifted.push(rndB[0]);

    let mut rndA_rndBshifted: Vec<u8> = Vec::with_capacity(64);
    rndA_rndBshifted.extend(&rndA);
    rndA_rndBshifted.extend(rndBshifted);

    let dk_rndA_rndBshifted_ref = aes_encrypt(&key, &rndA_rndBshifted)?;
    if dk_rndA_rndBshifted != dk_rndA_rndBshifted_ref {
        return Err(ServiceError::Unauthorized(
            "response does not match challenge!",
        ));
    }

    let rndAshifted_card = aes_decrypt(&key, ek_rndAshifted_card)?;

    let mut rndAshifted: Vec<u8> = Vec::with_capacity(132);
    rndAshifted.extend(&rndA[1..32]);
    rndAshifted.push(rndA[0]);

    if rndAshifted != rndAshifted_card {
        return Err(ServiceError::Unauthorized("challenge response failed!"));
    }

    Ok(Vec::new())
}
