use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::model::{Money, DB};

/// Represent a price of a product or category with a validity
///
/// The price with the newest `validity_start` lower than the current datetime is the current valid price.
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct Price {
    #[serde(with = "naive_date_time_serializer")]
    pub validity_start: NaiveDateTime,
    pub value: Money,
}

/// Serialize/Deserialize a datetime to/from only a date
pub mod naive_date_time_serializer {
    use chrono::{NaiveDate, NaiveDateTime};
    use serde::{de::Error, de::Unexpected, de::Visitor, Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.format("%Y-%m-%d").to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NaiveVisitor;

        impl<'de> Visitor<'de> for NaiveVisitor {
            type Value = NaiveDateTime;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("yyyy-mm-dd")
            }

            fn visit_str<E>(self, value: &str) -> Result<NaiveDateTime, E>
            where
                E: Error,
            {
                NaiveDate::parse_from_str(value, "%Y-%m-%d")
                    .map_err(|_| Error::invalid_value(Unexpected::Str(value), &"yyyy-mm-dd"))
                    .map(|d| d.and_hms(0, 0, 0))
            }
        }
        deserializer.deserialize_string(NaiveVisitor)
    }
}

/// Custom db loader for `Price`
///
/// Skip category id
impl
    diesel::Queryable<
        (
            diesel::sql_types::Uuid,
            diesel::sql_types::Timestamp,
            diesel::sql_types::Integer,
        ),
        DB,
    > for Price
{
    type Row = (Uuid, NaiveDateTime, Money);

    fn build(row: Self::Row) -> Self {
        Price {
            validity_start: row.1,
            value: row.2,
        }
    }
}
