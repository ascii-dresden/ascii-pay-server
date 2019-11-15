use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use std::fs::{self, File};
use std::path::Path;

use crate::core::{generate_uuid, DbConnection, Money, ServiceError, ServiceResult, DB};

// Encryption key for cookies
lazy_static::lazy_static! {
pub static ref IMAGE_PATH: String = std::env::var("IMAGE_PATH")
    .unwrap_or_else(|_| "img/".to_owned());
}

/// Represent a product
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub category: String,
    pub image: Option<String>,
    #[serde(default = "std::vec::Vec::new")]
    pub prices: Vec<Price>,
    pub current_price: Option<Money>,
}

/// Represent a price of a product with a validity
///
/// The price with the newest `validity_start` lower than the current datetime is the current valid price.
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Custom db loader for `Product`
///
/// Ignore price vec
impl
    diesel::Queryable<
        (
            diesel::sql_types::Text,
            diesel::sql_types::Text,
            diesel::sql_types::Text,
            diesel::sql_types::Nullable<diesel::sql_types::Text>,
        ),
        DB,
    > for Product
{
    type Row = (String, String, String, Option<String>);

    fn build(row: Self::Row) -> Self {
        Product {
            id: row.0,
            name: row.1,
            category: row.2,
            image: row.3,
            prices: vec![],
            current_price: None,
        }
    }
}

/// Custom db loader for `Price`
///
/// Skip product id
impl
    diesel::Queryable<
        (
            diesel::sql_types::Text,
            diesel::sql_types::Timestamp,
            diesel::sql_types::Integer,
        ),
        DB,
    > for Price
{
    type Row = (String, NaiveDateTime, Money);

    fn build(row: Self::Row) -> Self {
        Price {
            validity_start: row.1,
            value: row.2,
        }
    }
}

impl Product {
    /// Create a new product with the given name and category
    pub fn create(conn: &DbConnection, name: &str, category: &str) -> ServiceResult<Product> {
        use crate::core::schema::product::dsl;

        let p = Product {
            id: generate_uuid(),
            name: name.to_string(),
            category: category.to_string(),
            image: None,
            prices: vec![],
            current_price: None,
        };

        diesel::insert_into(dsl::product)
            .values((
                dsl::id.eq(&p.id),
                dsl::name.eq(&p.name),
                dsl::category.eq(&p.category),
            ))
            .execute(conn)?;

        Ok(p)
    }

    /// Save the current product data to the database
    ///
    /// This ignores all changes to the `prices` vec
    pub fn update(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::product::dsl;

        diesel::update(dsl::product.find(&self.id))
            .set((dsl::name.eq(&self.name), dsl::category.eq(&self.category)))
            .execute(conn)?;

        Ok(())
    }

    /// Add and save a new price to the product
    ///
    /// This updates the `prices` vec and the `current_price`
    pub fn add_price(
        &mut self,
        conn: &DbConnection,
        validity_start: NaiveDateTime,
        value: Money,
    ) -> ServiceResult<()> {
        use crate::core::schema::price::dsl;

        let p = Price {
            validity_start,
            value,
        };

        diesel::insert_into(dsl::price)
            .values((
                dsl::product.eq(&self.id),
                dsl::validity_start.eq(&p.validity_start),
                dsl::value.eq(&p.value),
            ))
            .execute(conn)?;

        self.prices.push(p);

        self.calc_current_price();

        Ok(())
    }

    /// Remove and save a price from the product by its `validity_start`
    ///
    /// This updates the `prices` vec and the `current_price`
    pub fn remove_price(
        &mut self,
        conn: &DbConnection,
        validity_start: NaiveDateTime,
    ) -> ServiceResult<()> {
        use crate::core::schema::price::dsl;

        let mut index = 0;
        for price in self.prices.iter() {
            if price.validity_start == validity_start {
                break;
            }
            index += 1;
        }

        diesel::delete(
            dsl::price.filter(
                dsl::product
                    .eq(&self.id)
                    .and(dsl::validity_start.eq(validity_start)),
            ),
        )
        .execute(conn)?;

        self.prices.remove(index);

        self.calc_current_price();

        Ok(())
    }

    /// Load the prices for this product
    ///
    /// This updates the `prices` vec and the `current_price`
    fn load_prices(&mut self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::price::dsl;

        let results = dsl::price
            .filter(dsl::product.eq(&self.id))
            .load::<Price>(conn)?;

        self.prices = results;

        self.calc_current_price();

        Ok(())
    }

    /// Calculate the `current_price` based on the `prices` vec
    fn calc_current_price(&mut self) {
        let now = Utc::now().naive_utc();

        let current = self
            .prices
            .iter()
            .filter(|p| p.validity_start <= now)
            .max_by(|p1, p2| p1.validity_start.cmp(&p2.validity_start));

        self.current_price = match current {
            Some(price) => Some(price.value),
            None => None,
        };
    }

    pub fn set_image(&mut self, conn: &DbConnection, file_extension: &str) -> ServiceResult<File> {
        use crate::core::schema::product::dsl;

        self.remove_image(&conn)?;

        let name = format!("{}.{}", generate_uuid(), file_extension);
        self.image = Some(name.clone());

        fs::create_dir_all(IMAGE_PATH.clone())?;
        let file = File::create(format!("{}/{}", IMAGE_PATH.clone(), name))?;

        diesel::update(dsl::product.find(&self.id))
            .set(dsl::image.eq(&self.image))
            .execute(conn)?;

        Ok(file)
    }

    pub fn remove_image(&mut self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::product::dsl;

        if let Some(name) = self.image.clone() {
            let p = format!("{}/{}", IMAGE_PATH.clone(), name);

            if Path::new(&p).exists() {
                fs::remove_file(p)?;
            }

            self.image = None;
            diesel::update(dsl::product.find(&self.id))
                .set(dsl::image.eq(&self.image))
                .execute(conn)?;
        }

        Ok(())
    }

    /// List all products
    pub fn all(conn: &DbConnection) -> ServiceResult<Vec<Product>> {
        use crate::core::schema::product::dsl;

        let mut results = dsl::product.load::<Product>(conn)?;

        for p in &mut results {
            p.load_prices(conn)?;
        }

        Ok(results)
    }

    /// Get a product by the `id`
    pub fn get(conn: &DbConnection, id: &str) -> ServiceResult<Product> {
        use crate::core::schema::product::dsl;

        let mut results = dsl::product.filter(dsl::id.eq(id)).load::<Product>(conn)?;

        let mut a = results.pop().ok_or_else(|| ServiceError::NotFound)?;

        a.load_prices(conn)?;

        Ok(a)
    }
}
