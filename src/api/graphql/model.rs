use async_graphql::ID;
use chrono::NaiveDateTime;

use crate::core::{self, naive_date_time_serializer, Money, Permission};

#[derive(SimpleObject, Serialize)]
pub struct LoginResult {
    pub token: String,
    pub authorization: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone, SimpleObject)]
pub struct Price {
    #[serde(with = "naive_date_time_serializer")]
    pub validity_start: NaiveDateTime,
    pub value: Money,
}

impl From<&core::Price> for Price {
    fn from(entity: &core::Price) -> Self {
        Self {
            validity_start: entity.validity_start,
            value: entity.value,
        }
    }
}

#[derive(InputObject, Deserialize)]
pub struct PriceInput {
    #[serde(with = "naive_date_time_serializer")]
    pub validity_start: NaiveDateTime,
    pub value: Money,
}

impl From<&PriceInput> for core::Price {
    fn from(entity: &PriceInput) -> Self {
        Self {
            validity_start: entity.validity_start,
            value: entity.value,
        }
    }
}

#[derive(SimpleObject, Serialize, Deserialize)]
pub struct Category {
    pub id: ID,
    pub name: String,
    pub prices: Vec<Price>,
    pub current_price: Option<Money>,
}

impl From<&core::Category> for Category {
    fn from(entity: &core::Category) -> Self {
        Self {
            id: entity.id.into(),
            name: entity.name.clone(),
            prices: entity.prices.iter().map(Price::from).collect(),
            current_price: entity.current_price,
        }
    }
}

#[derive(InputObject)]
pub struct CategoryCreateInput {
    pub name: String,
    pub prices: Vec<PriceInput>,
}

#[derive(InputObject)]
pub struct CategoryUpdateInput {
    pub id: ID,
    pub name: String,
    pub prices: Vec<PriceInput>,
}

#[derive(SimpleObject, Serialize, Deserialize)]
pub struct Product {
    pub id: ID,
    pub name: String,
    pub category: Option<Category>,
    pub image: Option<String>,
    pub prices: Vec<Price>,
    pub current_price: Option<Money>,
    pub barcode: Option<String>,
}

impl From<&core::Product> for Product {
    fn from(entity: &core::Product) -> Self {
        Self {
            id: entity.id.into(),
            name: entity.name.clone(),
            category: entity.category.as_ref().map(Category::from),
            image: entity.image.clone(),
            prices: entity.prices.iter().map(Price::from).collect(),
            current_price: entity.current_price,
            barcode: entity.barcode.clone(),
        }
    }
}

#[derive(InputObject)]
pub struct ProductCreateInput {
    pub name: String,
    pub category: Option<ID>,
    pub barcode: Option<String>,
    pub prices: Vec<PriceInput>,
}

#[derive(InputObject)]
pub struct ProductUpdateInput {
    pub id: ID,
    pub name: String,
    pub category: Option<ID>,
    pub barcode: Option<String>,
    pub prices: Vec<PriceInput>,
}

#[derive(SimpleObject, Serialize, Deserialize)]
pub struct Account {
    pub id: ID,
    pub credit: Money,
    pub minimum_credit: Money,
    pub name: String,
    pub mail: Option<String>,
    pub username: Option<String>,
    pub account_number: Option<String>,
    pub permission: Permission,
    pub receives_monthly_report: bool,
}

impl From<&core::Account> for Account {
    fn from(entity: &core::Account) -> Self {
        Self {
            id: entity.id.into(),
            credit: entity.credit,
            minimum_credit: entity.minimum_credit,
            name: entity.name.clone(),
            mail: entity.mail.clone(),
            username: entity.username.clone(),
            account_number: entity.account_number.clone(),
            permission: entity.permission,
            receives_monthly_report: entity.receives_monthly_report,
        }
    }
}
