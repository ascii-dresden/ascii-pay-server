use chrono::{Local, NaiveDateTime};
use diesel::prelude::*;
use uuid::Uuid;

use crate::utils::{generate_uuid, DatabaseConnection, Money, ServiceError, ServiceResult, DB};

use super::Price;

/// Represent a category
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: Uuid,
    #[serde(default = "std::string::String::new")]
    pub name: String,
    #[serde(default = "std::vec::Vec::new")]
    pub prices: Vec<Price>,
    pub current_price: Option<Money>,
}

/// Custom db loader for `Category`
///
/// Ignore price vec
impl diesel::Queryable<(diesel::sql_types::Uuid, diesel::sql_types::Text), DB> for Category {
    type Row = (Uuid, String);

    fn build(row: Self::Row) -> Self {
        Category {
            id: row.0,
            name: row.1,
            prices: vec![],
            current_price: None,
        }
    }
}

impl Category {
    /// Create a new category with the given name and category
    pub fn create(database_conn: &DatabaseConnection, name: &str) -> ServiceResult<Category> {
        use crate::model::schema::category::dsl;

        let p = Category {
            id: generate_uuid(),
            name: name.to_owned(),
            prices: vec![],
            current_price: None,
        };

        diesel::insert_into(dsl::category)
            .values((dsl::id.eq(&p.id), dsl::name.eq(&p.name)))
            .execute(database_conn)?;

        Ok(p)
    }

    /// Save the current category data to the database
    ///
    /// This ignores all changes to the `prices` vec
    pub fn update(&self, database_conn: &DatabaseConnection) -> ServiceResult<()> {
        use crate::model::schema::category::dsl;

        diesel::update(dsl::category.find(&self.id))
            .set(dsl::name.eq(&self.name))
            .execute(database_conn)?;

        Ok(())
    }

    /// Add and save a new price to the category
    ///
    /// This updates the `prices` vec and the `current_price`
    pub fn add_price(
        &mut self,
        database_conn: &DatabaseConnection,
        validity_start: NaiveDateTime,
        value: Money,
    ) -> ServiceResult<()> {
        use crate::model::schema::category_price::dsl;

        let p = Price {
            validity_start,
            value,
        };

        diesel::insert_into(dsl::category_price)
            .values((
                dsl::category_id.eq(&self.id),
                dsl::validity_start.eq(&p.validity_start),
                dsl::value.eq(&p.value),
            ))
            .execute(database_conn)?;

        self.prices.push(p);

        self.calc_current_price();

        Ok(())
    }

    /// Remove and save a price from the category by its `validity_start`
    ///
    /// This updates the `prices` vec and the `current_price`
    pub fn remove_price(
        &mut self,
        database_conn: &DatabaseConnection,
        validity_start: NaiveDateTime,
    ) -> ServiceResult<()> {
        use crate::model::schema::category_price::dsl;

        let mut index = 0;
        for price in self.prices.iter() {
            if price.validity_start == validity_start {
                break;
            }
            index += 1;
        }

        diesel::delete(
            dsl::category_price.filter(
                dsl::category_id
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
        use crate::model::schema::category_price::dsl;

        diesel::delete(dsl::category_price.filter(dsl::category_id.eq(&self.id)))
            .execute(database_conn)?;
        self.prices.clear();

        for p in new_prices {
            diesel::insert_into(dsl::category_price)
                .values((
                    dsl::category_id.eq(&self.id),
                    dsl::validity_start.eq(&p.validity_start),
                    dsl::value.eq(&p.value),
                ))
                .execute(database_conn)?;

            self.prices.push(p.clone());
        }

        self.calc_current_price();
        Ok(())
    }

    /// Load the prices for this category
    ///
    /// This updates the `prices` vec and the `current_price`
    fn load_prices(&mut self, database_conn: &DatabaseConnection) -> ServiceResult<()> {
        use crate::model::schema::category_price::dsl;

        let results = dsl::category_price
            .filter(dsl::category_id.eq(&self.id))
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

        current.map(|price| price.value)
    }

    /// List all categorys
    pub fn all(database_conn: &DatabaseConnection) -> ServiceResult<Vec<Category>> {
        use crate::model::schema::category::dsl;

        let mut results = dsl::category
            .order(dsl::name.asc())
            .load::<Category>(database_conn)?;

        for p in &mut results {
            p.load_prices(database_conn)?;
        }

        Ok(results)
    }

    /// Get a category by the `id`
    pub fn get(database_conn: &DatabaseConnection, id: Uuid) -> ServiceResult<Category> {
        use crate::model::schema::category::dsl;

        let mut results = dsl::category
            .filter(dsl::id.eq(id))
            .load::<Category>(database_conn)?;

        let mut category = results.pop().ok_or(ServiceError::NotFound)?;

        category.load_prices(database_conn)?;

        Ok(category)
    }
}