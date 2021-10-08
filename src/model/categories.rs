use diesel::prelude::*;
use uuid::Uuid;

use crate::model::schema::category;
use crate::utils::{generate_uuid, DatabaseConnection, Money, ServiceError, ServiceResult};

use super::StampType;

/// Represent a category
#[derive(
    Debug,
    Queryable,
    Insertable,
    Identifiable,
    AsChangeset,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Clone,
)]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "category"]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub price: Money,
    pub pay_with_stamps: StampType,
    pub give_stamps: StampType,
    pub ordering: Option<i32>,
}

impl Category {
    /// Create a new category with the given name and category
    pub fn create(
        database_conn: &DatabaseConnection,
        name: &str,
        price: Money,
    ) -> ServiceResult<Category> {
        use crate::model::schema::category::dsl;

        let p = Category {
            id: generate_uuid(),
            name: name.to_owned(),
            price,
            pay_with_stamps: StampType::None,
            give_stamps: StampType::None,
            ordering: None,
        };

        diesel::insert_into(dsl::category)
            .values(&p)
            .execute(database_conn)?;

        Ok(p)
    }

    /// Save the current category data to the database
    ///
    /// This ignores all changes to the `prices` vec
    pub fn update(&self, database_conn: &DatabaseConnection) -> ServiceResult<()> {
        use crate::model::schema::category::dsl;

        diesel::update(dsl::category.find(&self.id))
            .set(self)
            .execute(database_conn)?;

        Ok(())
    }

    /// List all categorys
    pub fn all(database_conn: &DatabaseConnection) -> ServiceResult<Vec<Category>> {
        use crate::model::schema::category::dsl;

        let results = dsl::category
            .order((dsl::ordering.asc(), dsl::name.asc()))
            .load::<Category>(database_conn)?;

        Ok(results)
    }

    /// Get a category by the `id`
    pub fn get(database_conn: &DatabaseConnection, id: Uuid) -> ServiceResult<Category> {
        use crate::model::schema::category::dsl;

        let mut results = dsl::category
            .filter(dsl::id.eq(id))
            .load::<Category>(database_conn)?;

        let category = results.pop().ok_or(ServiceError::NotFound)?;

        Ok(category)
    }
}
