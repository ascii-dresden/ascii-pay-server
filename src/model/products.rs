use diesel::prelude::*;
use log::info;
use std::fs::{self, File};
use std::path::Path;
use uuid::Uuid;

use super::{Category, StampType};
use crate::model::schema::product;
use crate::utils::{env, generate_uuid, DatabaseConnection, Money, ServiceError, ServiceResult};

/// Represent a product
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
#[table_name = "product"]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price: Option<Money>,
    pub pay_with_stamps: Option<StampType>,
    pub give_stamps: Option<StampType>,
    pub category_id: Uuid,
    pub image: Option<String>,
    pub barcode: Option<String>,
    pub ordering: Option<i32>,
}

impl Product {
    /// Create a new product with the given name and category
    pub fn create(
        database_conn: &DatabaseConnection,
        name: &str,
        category: &Category,
    ) -> ServiceResult<Product> {
        use crate::model::schema::product::dsl;

        let p = Product {
            id: generate_uuid(),
            name: name.to_owned(),
            category_id: category.id,
            price: None,
            pay_with_stamps: None,
            give_stamps: None,
            image: None,
            barcode: None,
            ordering: None,
        };

        diesel::insert_into(dsl::product)
            .values(&p)
            .execute(database_conn)?;

        Ok(p)
    }

    /// Save the current product data to the database
    ///
    /// This ignores all changes to the `prices` vec
    pub fn update(&self, database_conn: &DatabaseConnection) -> ServiceResult<()> {
        use crate::model::schema::product::dsl;

        diesel::update(dsl::product.find(&self.id))
            .set(self)
            .execute(database_conn)?;

        Ok(())
    }

    fn get_category(&self, database_conn: &DatabaseConnection) -> ServiceResult<Category> {
        Category::get(database_conn, self.category_id)
    }

    pub fn get_image(&self) -> ServiceResult<String> {
        if let Some(name) = self.image.clone() {
            let p = format!("{}/{}", env::IMAGE_PATH.as_str(), name);

            if Path::new(&p).exists() {
                return Ok(p);
            }
        }

        Err(ServiceError::NotFound)
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
        info!(
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
    pub fn all(database_conn: &DatabaseConnection) -> ServiceResult<Vec<(Product, Category)>> {
        use crate::model::schema::category::dsl as dsl2;
        use crate::model::schema::product::dsl;

        let results = dsl::product
            .order((dsl::ordering.asc(), dsl::name.asc()))
            .inner_join(dsl2::category)
            .load::<(Product, Category)>(database_conn)?;

        Ok(results)
    }

    /// Get a product by the `id`
    pub fn get(database_conn: &DatabaseConnection, id: Uuid) -> ServiceResult<(Product, Category)> {
        use crate::model::schema::category::dsl as dsl2;
        use crate::model::schema::product::dsl;

        let result = dsl::product
            .filter(dsl::id.eq(id))
            .inner_join(dsl2::category)
            .first::<(Product, Category)>(database_conn)?;

        Ok(result)
    }

    /// Get a product by the `barcode`
    pub fn get_by_barcode(
        database_conn: &DatabaseConnection,
        barcode: &str,
    ) -> ServiceResult<(Product, Category)> {
        use crate::model::schema::category::dsl as dsl2;
        use crate::model::schema::product::dsl;

        let result = dsl::product
            .filter(dsl::barcode.eq(barcode))
            .inner_join(dsl2::category)
            .first::<(Product, Category)>(database_conn)?;

        Ok(result)
    }
}
