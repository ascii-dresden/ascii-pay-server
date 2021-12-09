use block_modes::block_padding::ZeroPadding;
use block_modes::cipher::{BlockCipher, BlockDecrypt, BlockEncrypt, NewBlockCipher};
use block_modes::{BlockMode, Cbc};
use des::TdesEde2;
use generic_array::GenericArray;

use super::ServiceResult;

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

pub fn tdes_encrypt(key: &[u8], value: &[u8]) -> ServiceResult<Vec<u8>> {
    let mut v = Vec::with_capacity(16);
    v.extend(key);

    if key.len() == 8 {
        v.extend(key);
    }

    let key = GenericArray::from_slice(&v);

    let iv = GenericArray::from_slice(&hex!("00 00 00 00 00 00 00 00"));
    let cipher: Cbc<MiFareTdes, ZeroPadding> = Cbc::new(MiFareTdes::new(key), iv);

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
    let cipher: Cbc<MiFareTdes, ZeroPadding> = Cbc::new(MiFareTdes::new(key), iv);

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
