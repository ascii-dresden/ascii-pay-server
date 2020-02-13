use chrono::{Local, NaiveDateTime};
use diesel::prelude::*;
use uuid::Uuid;

use crate::core::{generate_uuid, DbConnection, Money, Price, ServiceError, ServiceResult, DB};

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
    pub fn create(conn: &DbConnection, name: &str) -> ServiceResult<Category> {
        use crate::core::schema::category::dsl;

        let p = Category {
            id: generate_uuid(),
            name: name.to_owned(),
            prices: vec![],
            current_price: None,
        };

        diesel::insert_into(dsl::category)
            .values((dsl::id.eq(&p.id), dsl::name.eq(&p.name)))
            .execute(conn)?;

        Ok(p)
    }

    /// Save the current category data to the database
    ///
    /// This ignores all changes to the `prices` vec
    pub fn update(&self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::category::dsl;

        diesel::update(dsl::category.find(&self.id))
            .set(dsl::name.eq(&self.name))
            .execute(conn)?;

        Ok(())
    }

    /// Add and save a new price to the category
    ///
    /// This updates the `prices` vec and the `current_price`
    pub fn add_price(
        &mut self,
        conn: &DbConnection,
        validity_start: NaiveDateTime,
        value: Money,
    ) -> ServiceResult<()> {
        use crate::core::schema::category_price::dsl;

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
            .execute(conn)?;

        self.prices.push(p);

        self.calc_current_price();

        Ok(())
    }

    /// Remove and save a price from the category by its `validity_start`
    ///
    /// This updates the `prices` vec and the `current_price`
    pub fn remove_price(
        &mut self,
        conn: &DbConnection,
        validity_start: NaiveDateTime,
    ) -> ServiceResult<()> {
        use crate::core::schema::category_price::dsl;

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
        .execute(conn)?;

        self.prices.remove(index);

        self.calc_current_price();

        Ok(())
    }

    pub fn update_prices(
        &mut self,
        conn: &DbConnection,
        new_prices: &[Price],
    ) -> ServiceResult<()> {
        use crate::core::schema::category_price::dsl;

        diesel::delete(dsl::category_price.filter(dsl::category_id.eq(&self.id))).execute(conn)?;
        self.prices.clear();

        for p in new_prices {
            diesel::insert_into(dsl::category_price)
                .values((
                    dsl::category_id.eq(&self.id),
                    dsl::validity_start.eq(&p.validity_start),
                    dsl::value.eq(&p.value),
                ))
                .execute(conn)?;

            self.prices.push(p.clone());
        }

        self.calc_current_price();
        Ok(())
    }

    /// Load the prices for this category
    ///
    /// This updates the `prices` vec and the `current_price`
    fn load_prices(&mut self, conn: &DbConnection) -> ServiceResult<()> {
        use crate::core::schema::category_price::dsl;

        let results = dsl::category_price
            .filter(dsl::category_id.eq(&self.id))
            .load::<Price>(conn)?;

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
            None => None,
        }
    }

    /// List all categorys
    pub fn all(conn: &DbConnection) -> ServiceResult<Vec<Category>> {
        use crate::core::schema::category::dsl;

        let mut results = dsl::category
            .order(dsl::name.asc())
            .load::<Category>(conn)?;

        for p in &mut results {
            p.load_prices(conn)?;
        }

        Ok(results)
    }

    /// Get a category by the `id`
    pub fn get(conn: &DbConnection, id: &Uuid) -> ServiceResult<Category> {
        use crate::core::schema::category::dsl;

        let mut results = dsl::category
            .filter(dsl::id.eq(id))
            .load::<Category>(conn)?;

        let mut category = results.pop().ok_or_else(|| ServiceError::NotFound)?;

        category.load_prices(conn)?;

        Ok(category)
    }
}
