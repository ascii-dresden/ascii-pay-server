use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use std::fs::{self, File};
use std::path::Path;

use crate::core::{
    generate_uuid, Category, DbConnection, Money, Price, Searchable, ServiceError, ServiceResult,
    DB,
};

// Encryption key for cookies
lazy_static::lazy_static! {
pub static ref IMAGE_PATH: String = std::env::var("IMAGE_PATH")
    .unwrap_or_else(|_| "img/".to_owned());
}

/// Represent a product
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub category: Option<Category>,
    pub image: Option<String>,
    #[serde(default = "std::vec::Vec::new")]
    pub prices: Vec<Price>,
    pub current_price: Option<Money>,
}

/// Custom db loader for `Product`
///
/// Ignore price vec
impl
    diesel::Queryable<
        (
            diesel::sql_types::Text,
            diesel::sql_types::Text,
            diesel::sql_types::Nullable<diesel::sql_types::Text>,
            diesel::sql_types::Nullable<diesel::sql_types::Text>,
        ),
        DB,
    > for Product
{
    type Row = (String, String, Option<String>, Option<String>);

    fn build(row: Self::Row) -> Self {
        let category = match row.2 {
            Some(id) => Some(Category {
                id,
                name: String::new(),
                prices: vec![],
                current_price: None,
            }),
            None => None,
        };

        Product {
            id: row.0,
            name: row.1,
            category,
            image: row.3,
            prices: vec![],
            current_price: None,
        }
    }
}

impl Product {
    /// Create a new product with the given name and category
    pub fn create(
        conn: &DbConnection,
        name: &str,
        category: Option<Category>,
    ) -> ServiceResult<Product> {
        use crate::core::schema::product::dsl;

        let category_id = match category.as_ref() {
            Some(category) => Some(category.id.to_owned()),
            None => None,
        };

        let p = Product {
            id: generate_uuid(),
            name: name.to_owned(),
            category,
            image: None,
            prices: vec![],
            current_price: None,
        };

        diesel::insert_into(dsl::product)
            .values((
                dsl::id.eq(&p.id),
                dsl::name.eq(&p.name),
                dsl::category.eq(&category_id),
            ))
            .execute(conn)?;

        Ok(p)
    }

    /// Save the current product data to the database
    ///
    /// This ignores all changes to the `prices` vec
    pub fn update(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::product::dsl;

        let category = match &self.category {
            Some(category) => Some(category.id.to_owned()),
            None => None,
        };

        diesel::update(dsl::product.find(&self.id))
            .set((dsl::name.eq(&self.name), dsl::category.eq(&category)))
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
        use crate::core::schema::product_price::dsl;

        let p = Price {
            validity_start,
            value,
        };

        diesel::insert_into(dsl::product_price)
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
        use crate::core::schema::product_price::dsl;

        let mut index = 0;
        for price in self.prices.iter() {
            if price.validity_start == validity_start {
                break;
            }
            index += 1;
        }

        diesel::delete(
            dsl::product_price.filter(
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

    fn load_category(&mut self, conn: &DbConnection) -> ServiceResult<()> {
        self.category = match &self.category {
            Some(category) => Some(Category::get(&conn, &category.id)?),
            None => None,
        };

        Ok(())
    }

    /// Load the prices for this product
    ///
    /// This updates the `prices` vec and the `current_price`
    fn load_prices(&mut self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::product_price::dsl;

        let results = dsl::product_price
            .filter(dsl::product.eq(&self.id))
            .load::<Price>(conn)?;

        self.prices = results;

        self.calc_current_price();

        Ok(())
    }

    /// Calculate the `current_price` based on the `prices` vec
    fn calc_current_price(&mut self) {
        self.current_price = self.get_price_at(&Utc::now().naive_utc());
    }

    pub fn get_price_at(&self, datetime: &NaiveDateTime) -> Option<Money> {
        let current = self
            .prices
            .iter()
            .filter(|p| p.validity_start <= *datetime)
            .max_by(|p1, p2| p1.validity_start.cmp(&p2.validity_start));

        match current {
            Some(price) => Some(price.value),
            None => match &self.category {
                Some(category) => category.get_price_at(datetime),
                None => None,
            },
        }
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
            p.load_category(conn)?;
            p.load_prices(conn)?;
        }

        Ok(results)
    }

    /// Get a product by the `id`
    pub fn get(conn: &DbConnection, id: &str) -> ServiceResult<Product> {
        use crate::core::schema::product::dsl;

        let mut results = dsl::product.filter(dsl::id.eq(id)).load::<Product>(conn)?;

        let mut p = results.pop().ok_or_else(|| ServiceError::NotFound)?;

        p.load_category(conn)?;
        p.load_prices(conn)?;

        Ok(p)
    }
}

impl Searchable for Product {
    fn contains(&self, search: &str) -> bool {
        if self.name.to_ascii_lowercase().contains(&search) {
            return true;
        }

        if let Some(category) = &self.category {
            if category.contains(&search) {
                return true;
            }
        }

        if let Some(current_price) = &self.current_price {
            if current_price.to_string().contains(&search) {
                return true;
            }
        }

        false
    }
}
