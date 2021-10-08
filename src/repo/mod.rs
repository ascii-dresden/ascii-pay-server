use async_graphql::OutputType;
use std::collections::HashMap;

mod accounts;
pub use accounts::*;

pub mod auth;
pub use auth::*;

pub mod categories;
pub use categories::*;

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

#[derive(Debug, Serialize, SimpleObject)]
pub struct SearchElementAccount {
    #[serde(flatten)]
    pub element: AccountOutput,
    pub highlight: HashMap<String, String>,
}
impl From<SearchElement<AccountOutput>> for SearchElementAccount {
    fn from(s: SearchElement<AccountOutput>) -> Self {
        Self {
            element: s.element,
            highlight: s.highlight,
        }
    }
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct SearchElementCategory {
    #[serde(flatten)]
    pub element: CategoryOutput,
    pub highlight: HashMap<String, String>,
}
impl From<SearchElement<CategoryOutput>> for SearchElementCategory {
    fn from(s: SearchElement<CategoryOutput>) -> Self {
        Self {
            element: s.element,
            highlight: s.highlight,
        }
    }
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct SearchElementProduct {
    #[serde(flatten)]
    pub element: ProductOutput,
    pub highlight: HashMap<String, String>,
}
impl From<SearchElement<ProductOutput>> for SearchElementProduct {
    fn from(s: SearchElement<ProductOutput>) -> Self {
        Self {
            element: s.element,
            highlight: s.highlight,
        }
    }
}
