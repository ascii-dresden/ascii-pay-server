use chrono::NaiveDateTime;

use crate::{
    model::{naive_date_time_serializer, Price},
    utils::Money,
};

#[derive(Debug, Deserialize, InputObject)]
pub struct PriceInput {
    #[serde(with = "naive_date_time_serializer")]
    pub validity_start: NaiveDateTime,
    pub value: Money,
}

impl From<PriceInput> for Price {
    fn from(entity: PriceInput) -> Self {
        Self {
            validity_start: entity.validity_start,
            value: entity.value,
        }
    }
}

#[derive(Debug, Serialize, SimpleObject)]
pub struct PriceOutput {
    #[serde(with = "naive_date_time_serializer")]
    pub validity_start: NaiveDateTime,
    pub value: Money,
}

impl From<Price> for PriceOutput {
    fn from(entity: Price) -> Self {
        Self {
            validity_start: entity.validity_start,
            value: entity.value,
        }
    }
}
