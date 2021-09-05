use std::collections::HashMap;

mod accounts;
pub use accounts::*;

pub mod auth;
use async_graphql::OutputType;
pub use auth::*;

pub mod categories;
pub use categories::*;

pub mod prices;
pub use prices::*;

pub mod products;
pub use products::*;

pub mod transactions;
pub use transactions::*;

pub mod authentication_token;
pub use authentication_token::*;

#[derive(Debug, Serialize, SimpleObject)]
pub struct SearchElement<T>
where
    T: Sync + Send + OutputType,
{
    #[serde(flatten)]
    pub element: T,
    pub highlight: HashMap<String, String>,
}

impl<T> SearchElement<T>
where
    T: Sync + Send + OutputType,
{
    pub fn new(element: T) -> Self {
        Self {
            element,
            highlight: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_highlight_ref(&mut self, key: &str, value: &str) {
        self.highlight.insert(key.to_owned(), value.to_owned());
    }

    pub fn add_highlight(&mut self, key: &str, value: String) {
        self.highlight.insert(key.to_owned(), value);
    }
}
