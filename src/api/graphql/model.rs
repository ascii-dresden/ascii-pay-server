use async_graphql::{Object, ID};
use chrono::NaiveDateTime;

use crate::core::{self, naive_date_time_serializer, Money};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct Price {
    #[serde(with = "naive_date_time_serializer")]
    pub validity_start: NaiveDateTime,
    pub value: Money,
}

#[Object]
impl Price {
    async fn validity_start(&self) -> &NaiveDateTime {
        &self.validity_start
    }

    async fn value(&self) -> &Money {
        &self.value
    }
}

impl From<&core::Price> for Price {
    fn from(entity: &core::Price) -> Self {
        Self {
            validity_start: entity.validity_start,
            value: entity.value,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Category {
    pub id: ID,
    pub name: String,
    pub prices: Vec<Price>,
    pub current_price: Option<Money>,
}

#[Object]
impl Category {
    async fn id(&self) -> &ID {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn prices(&self) -> &[Price] {
        &self.prices
    }

    async fn current_price(&self) -> &Option<Money> {
        &self.current_price
    }
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
    pub price: Option<Money>,
}

#[derive(Serialize, Deserialize)]
pub struct Product {
    pub id: ID,
    pub name: String,
    pub category: Option<Category>,
    pub image: Option<String>,
    pub prices: Vec<Price>,
    pub current_price: Option<Money>,
    pub barcode: Option<String>,
}

#[Object]
impl Product {
    async fn id(&self) -> &ID {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn category(&self) -> &Option<Category> {
        &self.category
    }

    async fn image(&self) -> &Option<String> {
        &self.image
    }

    async fn prices(&self) -> &[Price] {
        &self.prices
    }

    async fn current_price(&self) -> &Option<Money> {
        &self.current_price
    }

    async fn barcode(&self) -> &Option<String> {
        &self.barcode
    }
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
