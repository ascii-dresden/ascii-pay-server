use diesel::prelude::*;
use uuid::Uuid;

use aes::Aes128;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{Local, NaiveDateTime};
use rand_core::RngCore;
use std::io::Cursor;

use crate::core::schema::authentication_nfc;
use crate::core::schema::authentication_nfc_write_key;
use crate::core::{Account, DbConnection, ServiceError, ServiceResult};

/// Represent a nfc tag - nfc authentication for the given account
#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "authentication_nfc"]
#[primary_key(account_id)]
pub struct AuthenticationNfc {
    pub account_id: Uuid,
    pub card_id: String,
    key: Option<String>,
    secret: Option<String>,
}

#[derive(Debug, Queryable, Insertable, Identifiable)]
#[table_name = "authentication_nfc_write_key"]
#[primary_key(account_id)]
struct AuthenticationNfcWriteKey {
    account_id: Uuid,
    card_id: String,
}

impl AuthenticationNfc {
    fn update(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::authentication_nfc::dsl;

        diesel::update(self)
            .set((
                dsl::key.eq(self.key.as_ref()),
                dsl::secret.eq(self.secret.as_ref()),
            ))
            .execute(conn)?;

        Ok(())
    }

    fn add_write_key(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::authentication_nfc_write_key::dsl;

        let a = AuthenticationNfcWriteKey {
            account_id: self.account_id,
            card_id: self.card_id.clone(),
        };

        self.remove_write_key(conn)?;
        diesel::insert_into(dsl::authentication_nfc_write_key)
            .values(&a)
            .execute(conn)?;

        Ok(())
    }

    fn remove_write_key(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::authentication_nfc_write_key::dsl;

        diesel::delete(
            dsl::authentication_nfc_write_key.filter(dsl::account_id.eq(&self.account_id)),
        )
        .execute(conn)?;

        Ok(())
    }

    pub fn need_write_key(&self, conn: &DbConnection) -> ServiceResult<bool> {
        use crate::core::schema::authentication_nfc_write_key::dsl;

        let results = dsl::authentication_nfc_write_key
            .filter(
                dsl::account_id
                    .eq(&self.account_id)
                    .and(dsl::card_id.eq(&self.card_id)),
            )
            .load::<AuthenticationNfcWriteKey>(conn)?;

        Ok(!results.is_empty())
    }

    pub fn is_secure(&self) -> bool {
        self.key.is_some() && self.secret.is_some()
    }
}

/// Set the nfc as authentication method for the given account
pub fn register(
    conn: &DbConnection,
    account: &Account,
    card_id: &str,
    write_key: bool,
) -> ServiceResult<()> {
    use crate::core::schema::authentication_nfc::dsl;

    let a = AuthenticationNfc {
        account_id: account.id,
        card_id: card_id.to_owned(),
        key: None,
        secret: None,
    };

    remove(&conn, &account)?;
    diesel::insert_into(dsl::authentication_nfc)
        .values(&a)
        .execute(conn)?;

    if write_key {
        a.add_write_key(conn)?;
    }

    Ok(())
}

/// Remove the nfc authentication for the given account
pub fn remove(conn: &DbConnection, account: &Account) -> ServiceResult<()> {
    use crate::core::schema::authentication_nfc::dsl;
    use crate::core::schema::authentication_nfc_write_key::dsl as dsl2;

    diesel::delete(dsl::authentication_nfc.filter(dsl::account_id.eq(&account.id)))
        .execute(conn)?;

    diesel::delete(dsl2::authentication_nfc_write_key.filter(dsl2::account_id.eq(&account.id)))
        .execute(conn)?;

    Ok(())
}

pub fn get_nfcs(conn: &DbConnection, account: &Account) -> ServiceResult<Vec<AuthenticationNfc>> {
    use crate::core::schema::authentication_nfc::dsl;

    let results = dsl::authentication_nfc
        .filter(dsl::account_id.eq(&account.id))
        .load::<AuthenticationNfc>(conn)?;

    Ok(results)
}

/// Calculate CRC as descipted in ISO 14443.
#[allow(non_snake_case)]
fn crc_checksum(value: &[u8]) -> [u8; 2] {
    let mut wCrc = 0x6363;
    for b in value {
        let br = ((wCrc & 0xFF) as u8) ^ b;
        let br = br ^ (br << 4);
        let br_long = br as u32;
        wCrc = (wCrc >> 8) ^ (br_long << 8) ^ (br_long << 3) ^ (br_long >> 4);
    }

    [((wCrc) & 0xFF) as u8, ((wCrc >> 8) & 0xFF) as u8]
}

/// Create a new challenge. A challenge contains the current timestamp, 
/// a random byte sequence and a crc checksum. 
/// Afterwords, the challenge is signed/encrypted with aes/cbc.
fn generate_challenge() -> ServiceResult<String> {
    let mut buffer: Vec<u8> = Vec::new();

    // Generate current timestamp to validate challenge
    let now = Local::now().naive_local().timestamp();
    buffer.write_i64::<LittleEndian>(now)?;

    // Generate random challenge
    let mut data = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut data);
    buffer.extend(&data);

    // Checksum to verfiy integrity
    buffer.extend(&crc_checksum(&buffer));

    type Aes128Cbc = Cbc<Aes128, Pkcs7>;
    let key = hex!("000102030405060708090a0b0c0d0e0f");
    let iv = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
    let cipher = Aes128Cbc::new_from_slices(&key, &iv)?;

    // Sign challenge
    let ciphertext = cipher.encrypt_vec(&buffer);

    Ok(base64::encode(&ciphertext))
}

/// Verify if a challenge is valid. First it decrypts the challenge 
/// and verifies the integrity with the crc checksum.
/// Afterwords it checks if the timestamp is not older than 2 minutes.
fn verify_challenge(challenge: &str) -> ServiceResult<bool> {
    let ciphertext = base64::decode(challenge)?;

    // Verify integrity
    type Aes128Cbc = Cbc<Aes128, Pkcs7>;
    let key = hex!("000102030405060708090a0b0c0d0e0f");
    let iv = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
    let cipher = Aes128Cbc::new_from_slices(&key, &iv)?;

    let buffer = cipher.decrypt_vec(&ciphertext)?;

    let checksum = crc_checksum(&buffer[0..24]);
    if buffer[24..26] != checksum {
        return Ok(false);
    }

    let mut cursor = Cursor::new(buffer);

    let timestamp = cursor.read_i64::<LittleEndian>()?;
    let now = Local::now().naive_local();
    let challenge_time = NaiveDateTime::from_timestamp(timestamp, 0);

    // Check timestamp, if older than 2 minutes it is invalid
    if (now - challenge_time).num_minutes() >= 2 {
        return Ok(false);
    }

    Ok(true)
}

/// Create the response for the given challenge and card secret.
/// The challenge is base64 encoded.
/// 
/// To create the response each byte of the challenge is xor-ed with the secret.
/// If the challenge is longer than the secret, than the secret will repeat itself.
/// 
/// The result is base64 encoded.
fn create_response(secret: &[u8], challenge: &str) -> ServiceResult<String> {
    let challenge = base64::decode(challenge)?;

    let mut response: Vec<u8> = Vec::with_capacity(challenge.len());

    for (i, c) in challenge.iter().enumerate() {
        response.push(c | secret[i % secret.len()]);
    }

    Ok(base64::encode(&response))
}

/// Convienient function that verifies the validity of the challange 
/// and afterwords checks if the response matches the challenge.
fn verify_challenge_response(secret: &[u8], challenge: &str, response: &str) -> ServiceResult<bool> {
    Ok(verify_challenge(challenge)? && create_response(secret, challenge)? == response)
}

/// Convert byte sequence to space separated hex string.
fn bytes_to_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|x| format!("{:02X}", x))
        .collect::<Vec<String>>()
        .join(" ")
}

/// Convert space separate hex string to byte vector.
fn str_to_bytes(s: &str) -> ServiceResult<Vec<u8>> {
    fn m(x: &str) -> ServiceResult<u8> {
        Ok(u8::from_str_radix(x, 16)?)
    }

    s.split(' ').map(m).collect()
}

/// Generate a random byte sequence of length `length`.
fn generate_key(length: usize) -> Vec<u8> {
    let mut data = vec![0u8; length];
    rand::thread_rng().fill_bytes(&mut data);
    data
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum NfcResult {
    Ok { account: Account },
    WriteKey { key: String, secret: String },
    AuthenticationRequested { key: String, challenge: String },
}

/// Get account by nfc.
/// Return `ServiceError` if no account is registered for given nfc.
pub fn get(conn: &DbConnection, card_id: &str) -> ServiceResult<NfcResult> {
    use crate::core::schema::authentication_nfc::dsl;

    let mut results = dsl::authentication_nfc
        .filter(dsl::card_id.eq(card_id))
        .limit(1)
        .load::<AuthenticationNfc>(conn)?;

    let mut entry = results.pop().ok_or_else(|| ServiceError::NotFound)?;

    if entry.need_write_key(&conn)? {
        let key = bytes_to_string(&generate_key(16));
        let secret = bytes_to_string(&generate_key(16));

        entry.key = Some(key.clone());
        entry.secret = Some(secret.clone());

        entry.update(&conn)?;
        entry.remove_write_key(&conn)?;

        return Ok(NfcResult::WriteKey { key, secret });
    }

    if entry.secret.is_some() {
        match entry.key {
            Some(key) => Ok(NfcResult::AuthenticationRequested {
                key,
                challenge: generate_challenge()?,
            }),
            None => Err(ServiceError::InternalServerError(
                "nfc key mismatch",
                "".to_owned(),
            )),
        }
    } else {
        let account = Account::get(conn, &entry.account_id)?;
        Ok(NfcResult::Ok { account })
    }
}

/// Get account by nfc.
/// Return `ServiceError` if no account is registered for given nfc.
pub fn get_challenge_response(
    conn: &DbConnection,
    card_id: &str,
    challenge: &str,
    response: &str,
) -> ServiceResult<Account> {
    use crate::core::schema::authentication_nfc::dsl;

    let mut results = dsl::authentication_nfc
        .filter(dsl::card_id.eq(card_id))
        .limit(1)
        .load::<AuthenticationNfc>(conn)?;

    let entry = results.pop().ok_or_else(|| ServiceError::NotFound)?;

    if let Some(secret) = entry.secret {
        let secret = str_to_bytes(&secret)?;
        if verify_challenge_response(&secret, challenge, response)? {
            let account = Account::get(conn, &entry.account_id)?;
            return Ok(account);
        }
        Err(ServiceError::Unauthorized)
    } else {
        Err(ServiceError::BadRequest(
            "Illegal secret",
            "No secret is required for this card!".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_response() -> ServiceResult<()> {
        let secret = generate_key(16);
        let challenge = generate_challenge()?;

        let response = create_response(&secret, &challenge)?;
        assert_eq!(
            verify_challenge_response(&secret, &challenge, &response)?,
            true
        );

        Ok(())
    }
}
