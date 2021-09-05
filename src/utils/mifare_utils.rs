use aes::NewBlockCipher;
use block_modes::block_padding::ZeroPadding;
use block_modes::{BlockMode, Cbc};
use des::TdesEde2;
use generic_array::GenericArray;

use super::ServiceResult;

pub fn tdes_encrypt(key: &[u8], value: &[u8]) -> ServiceResult<Vec<u8>> {
    let mut v = Vec::with_capacity(16);
    v.extend(key);

    if key.len() == 8 {
        v.extend(key);
    }

    let key = GenericArray::from_slice(&v);

    let iv = GenericArray::from_slice(&hex!("00 00 00 00 00 00 00 00"));
    let cipher: Cbc<TdesEde2, ZeroPadding> = Cbc::new(TdesEde2::new(key), iv);

    Ok(cipher.encrypt_vec(value))
}

pub fn tdes_decrypt(key: &[u8], value: &[u8]) -> ServiceResult<Vec<u8>> {
    let mut v = Vec::with_capacity(16);
    v.extend(key);

    if key.len() == 8 {
        v.extend(key);
    }

    let key = GenericArray::from_slice(&v);

    let iv = GenericArray::from_slice(&hex!("00 00 00 00 00 00 00 00"));
    let cipher: Cbc<TdesEde2, ZeroPadding> = Cbc::new(TdesEde2::new(key), iv);

    Ok(cipher.decrypt_vec(value)?)
}

pub fn is_key_2des(key: &[u8]) -> bool {
    if key.len() == 8 {
        return false;
    }

    if key.len() == 16 && key[0..8] == key[8..16] {
        return false;
    }

    true
}

pub fn generate_key() -> [u8; 8] {
    use rand_core::RngCore;

    let mut data = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut data);
    data
}
