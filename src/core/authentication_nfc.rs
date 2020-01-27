use diesel::prelude::*;
use uuid::Uuid;

use rand_core::RngCore;
use byteorder::{WriteBytesExt, ReadBytesExt, LittleEndian};
use block_modes::{BlockMode, Cbc};
use block_modes::block_padding::Pkcs7;
use aes::Aes128;
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
                dsl::secret.eq(self.secret.as_ref())
            ))
            .execute(conn)?;

        Ok(())
    }

    fn add_write_key(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::authentication_nfc_write_key::dsl;

        let a = AuthenticationNfcWriteKey {
            account_id: self.account_id.clone(),
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

        diesel::delete(dsl::authentication_nfc_write_key.filter(dsl::account_id.eq(&self.account_id)))
            .execute(conn)?;

        Ok(())
    }

    pub fn need_write_key(&self, conn: &DbConnection) -> ServiceResult<bool> {
        use crate::core::schema::authentication_nfc_write_key::dsl;

        let results = dsl::authentication_nfc_write_key
            .filter(dsl::account_id.eq(&self.account_id).and(dsl::card_id.eq(&self.card_id)))
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

fn generate_challenge() -> ServiceResult<String> {    
    let mut buffer: Vec<u8> = Vec::new();

    // Generate current timestamp to validate challenge
    let now = chrono::Utc::now().naive_utc().timestamp();
    buffer.write_i64::<LittleEndian>(now)?;

    // Generate random challange
    let mut data = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut data);
    buffer.extend(&data);

    type Aes128Cbc = Cbc<Aes128, Pkcs7>;
    let key = hex!("000102030405060708090a0b0c0d0e0f");
    let iv = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
    let cipher = Aes128Cbc::new_var(&key, &iv)?;

    // Sign challenge
    let ciphertext = cipher.encrypt_vec(&buffer);

    Ok(base64::encode(&ciphertext))
}

fn verify_challenge(challenge: &str) -> ServiceResult<bool> {
    let ciphertext = base64::decode(challenge)?;

    type Aes128Cbc = Cbc<Aes128, Pkcs7>;
    let key = hex!("000102030405060708090a0b0c0d0e0f");
    let iv = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
    let cipher = Aes128Cbc::new_var(&key, &iv)?;

    let buffer = cipher.decrypt_vec(&ciphertext)?;
    let mut cursor = Cursor::new(buffer);

    let timestamp = cursor.read_i64::<LittleEndian>()?;
    let now = chrono::Utc::now().naive_utc();
    let challenge_time = chrono::NaiveDateTime::from_timestamp(timestamp, 0);

    // Check timestamp
    if (now - challenge_time).num_minutes() >= 2 {
        return Ok(false);
    }

    // TODO verify random bytes

    Ok(true)
}

fn create_response(_secret: &str, challenge: &str) -> ServiceResult<String> {
    // TODO
    Ok(challenge.to_owned())
}

fn verify_challenge_response(secret: &str, challenge: &str, response: &str) -> ServiceResult<bool> {
    Ok(verify_challenge(challenge)? && &create_response(secret, challenge)? == response)
}

fn bytes_to_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|x| format!("{:02X}", x))
        .collect::<Vec<String>>()
        .join(" ")
}

fn str_to_bytes(s: &str) -> Vec<u8> {
    s.split(' ')
        .map(|x| u8::from_str_radix(x, 16).unwrap_or(0))
        .collect()
}

fn generate_key(length: usize) -> Vec<u8> {
    let mut data = vec![0u8; length];
    rand::thread_rng().fill_bytes(&mut data);
    data
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum NfcResult {
    Ok { account: Account },
    WriteKey { key: String, secret: String},
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

        return Ok(NfcResult::WriteKey {
            key, 
            secret,
        })
    }

    if let Some(_) = entry.secret {
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
