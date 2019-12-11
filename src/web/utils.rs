use std::collections::HashMap;

use actix_web::http::header::COOKIE;
use actix_web::web::HttpRequest;
use handlebars::{Handlebars, RenderError};
use serde::ser::Serialize;
use serde_json::value::Value;

use crate::core::Account;
use crate::web::identity_policy::{LoggedAccount};

/// Helper to convert empty strings to `None` values
pub trait EmptyToNone<T> {
    fn empty_to_none(&self) -> Option<T>;
}

impl EmptyToNone<String> for Option<String> {
    fn empty_to_none(&self) -> Option<String> {
        match self {
            None => None,
            Some(s) => {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }
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
        let mut theme = "light";
        if let Some(header_value) = request.headers().get(COOKIE) {
            if let Ok(header_str) = header_value.to_str() {
                if header_str.contains("theme=dark") {
                    theme = "dark";
                }
            }
        }
        HbData {
            theme: theme.to_owned(),
            logged_account: None,
            extra: HashMap::new(),
        }
    }

    pub fn with_account(mut self, logged_account: LoggedAccount) -> Self {
        self.logged_account = Some(logged_account.account);
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
