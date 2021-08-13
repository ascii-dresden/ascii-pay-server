use std::collections::HashMap;

use actix_http::httpmessage::HttpMessage;
use actix_web::http::header::COOKIE;
use actix_web::web::HttpRequest;
use handlebars::{Handlebars, RenderError};
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use serde_json::value::Value;

use aes::Aes128;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};

use crate::core::{Account, ServiceResult};

lazy_static::lazy_static! {
    pub static ref COOKIE_KEY: [u8; 16] = hex!("000102030405060708090a0b0c0d0e0f");
    pub static ref COOKIE_IV: [u8; 16] = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
}

/// Helper to convert empty strings to `None` values
pub trait EmptyToNone<T> {
    fn empty_to_none(&self) -> Option<T>;
}

impl EmptyToNone<String> for Option<String> {
    fn empty_to_none(&self) -> Option<String> {
        match self {
            Some(s) => s.empty_to_none(),
            None => None,
        }
    }
}
impl EmptyToNone<String> for String {
    fn empty_to_none(&self) -> Option<String> {
        if self.is_empty() {
            None
        } else {
            Some(self.clone())
        }
    }
}

pub trait IsJson {
    fn is_json(&self) -> bool;
}
impl IsJson for HttpRequest {
    fn is_json(&self) -> bool {
        self.content_type() == "application/json"
    }
}

/// Helper to deserialize search queries
#[derive(Deserialize)]
pub struct Search {
    pub search: Option<String>,
}

#[derive(Serialize)]
pub struct HbData {
    theme: String,
    logged_account: Option<Account>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

impl HbData {
    pub fn new(request: &HttpRequest) -> Self {
        let mut theme = "auto";
        if let Some(header_value) = request.headers().get(COOKIE) {
            if let Ok(header_str) = header_value.to_str() {
                if header_str.contains("theme=dark") {
                    theme = "dark";
                }
                if header_str.contains("theme=light") {
                    theme = "light";
                }
            }
        }
        HbData {
            theme: theme.to_owned(),
            logged_account: None,
            extra: HashMap::new(),
        }
    }

    pub fn with_account(mut self, account: Account) -> Self {
        self.logged_account = Some(account);
        self
    }

    pub fn with_data<T>(mut self, key: &str, value: &T) -> Self
    where
        T: Serialize,
    {
        self.extra.insert(key.to_owned(), json!(value));
        self
    }

    pub fn render(self, hb: &Handlebars, page: &str) -> Result<String, RenderError> {
        hb.render(page, &self)
    }
}

pub fn parse_obj_from_token<T>(token: &str) -> ServiceResult<T>
where
    T: DeserializeOwned,
{
    type Aes128Cbc = Cbc<Aes128, Pkcs7>;
    let cipher = Aes128Cbc::new_from_slices(COOKIE_KEY.as_ref(), COOKIE_IV.as_ref())?;

    let ciphertext = base64::decode(token)?;
    let buffer = cipher.decrypt_vec(&ciphertext)?;
    let obj: T = serde_json::from_slice(&buffer)?;

    Ok(obj)
}

pub fn create_token_from_obj<T>(obj: &T) -> ServiceResult<String>
where
    T: Serialize,
{
    type Aes128Cbc = Cbc<Aes128, Pkcs7>;
    let cipher = Aes128Cbc::new_from_slices(COOKIE_KEY.as_ref(), COOKIE_IV.as_ref())?;

    let buffer = serde_json::to_vec(obj)?;
    let ciphertext = cipher.encrypt_vec(&buffer);

    Ok(base64::encode(&ciphertext))
}
