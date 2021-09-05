use chrono::{Local, NaiveDateTime};
use diesel::prelude::*;
use std::fs::{self, File};
use std::path::Path;
use uuid::Uuid;

use crate::utils::{
    env, generate_uuid, DatabaseConnection, Money, ServiceError, ServiceResult, DB,
};

use super::{Category, Price};

/// Represent a product
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub category: Option<Category>,
    pub image: Option<String>,
    #[serde(default = "std::vec::Vec::new")]
    pub prices: Vec<Price>,
    pub current_price: Option<Money>,
    pub barcode: Option<String>,
}

/// Custom db loader for `Product`
///
/// Ignore price vec
impl
    diesel::Queryable<
        (
            diesel::sql_types::Uuid,
            diesel::sql_types::Text,
            diesel::sql_types::Nullable<diesel::sql_types::Uuid>,
            diesel::sql_types::Nullable<diesel::sql_types::Text>,
            diesel::sql_types::Nullable<diesel::sql_types::Text>,
        ),
        DB,
    > for Product
{
    type Row = (Uuid, String, Option<Uuid>, Option<String>, Option<String>);

    fn build(row: Self::Row) -> Self {
        let category = row.2.map(|id| Category {
            id,
            name: String::new(),
            prices: vec![],
            current_price: None,
        });

        Product {
            id: row.0,
            name: row.1,
            category,
            image: row.3,
            prices: vec![],
            current_price: None,
            barcode: row.4,
        }
    }
}

impl Product {
    /// Create a new product with the given name and category
    pub fn create(
        database_conn: &DatabaseConnection,
        name: &str,
        category: Option<Category>,
    ) -> ServiceResult<Product> {
        use crate::model::schema::product::dsl;

        let category_id = category.as_ref().map(|category| category.id.to_owned());

        let p = Product {
            id: generate_uuid(),
            name: name.to_owned(),
            category,
            image: None,
            prices: vec![],
            current_price: None,
            barcode: None,
        };

        diesel::insert_into(dsl::product)
            .values((
                dsl::id.eq(&p.id),
                dsl::name.eq(&p.name),
                dsl::category.eq(&category_id),
            ))
            .execute(database_conn)?;

        Ok(p)
    }

    /// Save the current product data to the database
    ///
    /// This ignores all changes to the `prices` vec
    pub fn update(&self, database_conn: &DatabaseConnection) -> ServiceResult<()> {
        use crate::model::schema::product::dsl;

        let category = self
            .category
            .as_ref()
            .map(|category| category.id.to_owned());

        diesel::update(dsl::product.find(&self.id))
            .set((
                dsl::name.eq(&self.name),
                dsl::category.eq(&category),
                dsl::barcode.eq(&self.barcode),
            ))
            .execute(database_conn)?;

        Ok(())
    }

    /// Add and save a new price to the product
    ///
    /// This updates the `prices` vec and the `current_price`
    pub fn add_price(
        &mut self,
        database_conn: &DatabaseConnection,
        validity_start: NaiveDateTime,
        value: Money,
    ) -> ServiceResult<()> {
        use crate::model::schema::product_price::dsl;

        let p = Price {
            validity_start,
            value,
        };

        diesel::insert_into(dsl::product_price)
            .values((
                dsl::product_id.eq(&self.id),
                dsl::validity_start.eq(&p.validity_start),
                dsl::value.eq(&p.value),
            ))
            .execute(database_conn)?;

        self.prices.push(p);

        self.calc_current_price();

        Ok(())
    }

    /// Remove and save a price from the product by its `validity_start`
    ///
    /// This updates the `prices` vec and the `current_price`
    pub fn remove_price(
        &mut self,
        database_conn: &DatabaseConnection,
        validity_start: NaiveDateTime,
    ) -> ServiceResult<()> {
        use crate::model::schema::product_price::dsl;

        let mut index = 0;
        for price in self.prices.iter() {
            if price.validity_start == validity_start {
                break;
            }
            index += 1;
        }

        diesel::delete(
            dsl::product_price.filter(
                dsl::product_id
                    .eq(&self.id)
                    .and(dsl::validity_start.eq(validity_start)),
            ),
        )
        .execute(database_conn)?;

        self.prices.remove(index);

        self.calc_current_price();

        Ok(())
    }

    pub fn update_prices(
        &mut self,
        database_conn: &DatabaseConnection,
        new_prices: &[Price],
    ) -> ServiceResult<()> {
        use crate::model::schema::product_price::dsl;

        diesel::delete(dsl::product_price.filter(dsl::product_id.eq(&self.id)))
            .execute(database_conn)?;
        self.prices.clear();

        for p in new_prices {
            diesel::insert_into(dsl::product_price)
                .values((
                    dsl::product_id.eq(&self.id),
                    dsl::validity_start.eq(&p.validity_start),
                    dsl::value.eq(&p.value),
                ))
                .execute(database_conn)?;

            self.prices.push(p.clone());
        }

        self.calc_current_price();
        Ok(())
    }

    fn load_category(&mut self, database_conn: &DatabaseConnection) -> ServiceResult<()> {
        self.category = match &self.category {
            Some(category) => Some(Category::get(database_conn, &category.id)?),
            None => None,
        };

        Ok(())
    }

    /// Load the prices for this product
    ///
    /// This updates the `prices` vec and the `current_price`
    fn load_prices(&mut self, database_conn: &DatabaseConnection) -> ServiceResult<()> {
        use crate::model::schema::product_price::dsl;

        let results = dsl::product_price
            .filter(dsl::product_id.eq(&self.id))
            .load::<Price>(database_conn)?;

        self.prices = results;

        self.calc_current_price();

        Ok(())
    }

    /// Calculate the `current_price` based on the `prices` vec
    fn calc_current_price(&mut self) {
        self.current_price = self.get_price_at(&Local::now().naive_local());
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

    pub fn set_image(
        &mut self,
        database_conn: &DatabaseConnection,
        file_extension: &str,
    ) -> ServiceResult<File> {
        use crate::model::schema::product::dsl;

        self.remove_image(database_conn)?;

        let name = format!("{}.{}", generate_uuid(), file_extension);
        self.image = Some(name.clone());

        fs::create_dir_all(env::IMAGE_PATH.as_str())?;
        let file = File::create(format!("{}/{}", env::IMAGE_PATH.as_str(), name))?;
        println!(
            "Save image '{}'",
            format!("{}/{}", env::IMAGE_PATH.as_str(), name)
        );

        diesel::update(dsl::product.find(&self.id))
            .set(dsl::image.eq(&self.image))
            .execute(database_conn)?;

        Ok(file)
    }

    pub fn remove_image(&mut self, database_conn: &DatabaseConnection) -> ServiceResult<()> {
        use crate::model::schema::product::dsl;

        if let Some(name) = self.image.clone() {
            let p = format!("{}/{}", env::IMAGE_PATH.as_str(), name);

            if Path::new(&p).exists() {
                fs::remove_file(p)?;
            }

            self.image = None;
            diesel::update(dsl::product.find(&self.id))
                .set(dsl::image.eq(&self.image))
                .execute(database_conn)?;
        }

        Ok(())
    }

    /// List all products
    pub fn all(database_conn: &DatabaseConnection) -> ServiceResult<Vec<Product>> {
        use crate::model::schema::product::dsl;

        let mut results = dsl::product
            .order(dsl::name.asc())
            .load::<Product>(database_conn)?;

        for p in &mut results {
            p.load_category(database_conn)?;
            p.load_prices(database_conn)?;
        }

        Ok(results)
    }

    /// Get a product by the `id`
    pub fn get(database_conn: &DatabaseConnection, id: &Uuid) -> ServiceResult<Product> {
        use crate::model::schema::product::dsl;

        let mut results = dsl::product
            .filter(dsl::id.eq(id))
            .load::<Product>(database_conn)?;

        let mut p = results.pop().ok_or(ServiceError::NotFound)?;

        p.load_category(database_conn)?;
        p.load_prices(database_conn)?;

        Ok(p)
    }
    /// Get a product by the `id`
    pub fn get_by_barcode(
        database_conn: &DatabaseConnection,
        barcode: &str,
    ) -> ServiceResult<Product> {
        use crate::model::schema::product::dsl;

        let mut results = dsl::product
            .filter(dsl::barcode.eq(barcode))
            .load::<Product>(database_conn)?;

        let mut p = results.pop().ok_or(ServiceError::NotFound)?;

        p.load_category(database_conn)?;
        p.load_prices(database_conn)?;

        Ok(p)
    }
}
