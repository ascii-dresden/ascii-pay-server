use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::core::{generate_uuid, DbConnection, Money, ServiceError, DB};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub prices: Vec<Price>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Price {
    pub validity_start: NaiveDateTime,
    pub value: Money,
}

impl diesel::Queryable<(diesel::sql_types::Text, diesel::sql_types::Text), DB> for Product {
    type Row = (String, String);

    fn build(row: Self::Row) -> Self {
        Product {
            id: row.0,
            name: row.1,
            prices: vec![],
        }
    }
}

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
    pub fn create(conn: &DbConnection, name: &str) -> Result<Product, ServiceError> {
        use crate::core::schema::product::dsl;

        let p = Product {
            id: generate_uuid(),
            name: name.to_string(),
            prices: vec![],
        };

        diesel::insert_into(dsl::product)
            .values((dsl::id.eq(&p.id), dsl::name.eq(&p.name)))
            .execute(conn)?;

        Ok(p)
    }

    pub fn update(&self, conn: &DbConnection) -> Result<(), ServiceError> {
        use crate::core::schema::product::dsl;

        diesel::update(dsl::product)
            .set(dsl::name.eq(&self.name))
            .execute(conn)?;

        Ok(())
    }

    pub fn add_price(
        &mut self,
        conn: &DbConnection,
        validity_start: NaiveDateTime,
        value: Money,
    ) -> Result<(), ServiceError> {
        use crate::core::schema::price::dsl;

        let p = Price {
            validity_start,
            value,
        };

        diesel::insert_into(dsl::price)
            .values((
                dsl::validity_start.eq(&p.validity_start),
                dsl::value.eq(&p.value),
            ))
            .execute(conn)?;

        self.prices.push(p);

        Ok(())
    }

    fn load_prices(&mut self, conn: &DbConnection) -> Result<(), ServiceError> {
        use crate::core::schema::price::dsl;

        let results = dsl::price
            .filter(dsl::product.eq(&self.id))
            .load::<Price>(conn)?;

        self.prices = results;

        Ok(())
    }

    pub fn all(conn: &DbConnection) -> Result<Vec<Product>, ServiceError> {
        use crate::core::schema::product::dsl;

        let mut results = dsl::product.load::<Product>(conn)?;

        for p in &mut results {
            p.load_prices(conn)?;
        }

        Ok(results)
    }

    pub fn get(conn: &DbConnection, id: &str) -> Result<Product, ServiceError> {
        use crate::core::schema::product::dsl;

        let mut results = dsl::product.filter(dsl::id.eq(id)).load::<Product>(conn)?;

        let mut a = results.pop().ok_or_else(|| ServiceError::NotFound)?;

        a.load_prices(conn)?;

        Ok(a)
    }
}
