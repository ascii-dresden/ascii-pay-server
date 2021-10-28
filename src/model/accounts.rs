use diesel::prelude::*;
use uuid::Uuid;

use crate::model::schema::account;
use crate::utils::{generate_uuid, DatabaseConnection, Money, ServiceError, ServiceResult};

use super::enums::Permission;

/// Represent a account
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
#[table_name = "account"]
pub struct Account {
    pub id: Uuid,
    pub credit: Money,
    pub minimum_credit: Money,
    pub name: String,
    pub mail: String,
    pub username: String,
    pub account_number: String,
    pub permission: Permission,
    pub use_digital_stamps: bool,
    pub coffee_stamps: i32,
    pub bottle_stamps: i32,
    pub receives_monthly_report: bool,
}

impl Account {
    /// Create a new account with the given permission level
    pub fn create(
        database_conn: &DatabaseConnection,
        name: &str,
        permission: Permission,
    ) -> ServiceResult<Account> {
        use crate::model::schema::account::dsl;

        let a = Account {
            id: generate_uuid(),
            credit: 0,
            minimum_credit: 0,
            name: name.to_owned(),
            mail: String::new(),
            username: String::new(),
            account_number: String::new(),
            permission,
            receives_monthly_report: false,
            use_digital_stamps: true,
            coffee_stamps: 0,
            bottle_stamps: 0,
        };

        if !a.exist_conflicting_account(database_conn)? {
            return Err(ServiceError::InternalServerError(
                "Conflicting account settings",
                "The given account settings conflict with the other existing accounts".to_owned(),
            ));
        }

        diesel::insert_into(dsl::account)
            .values(&a)
            .execute(database_conn)?;

        Ok(a)
    }

    /// Save the current account data to the database
    pub fn update(&self, database_conn: &DatabaseConnection) -> ServiceResult<()> {
        use crate::model::schema::account::dsl;

        if !self.exist_conflicting_account(database_conn)? {
            return Err(ServiceError::InternalServerError(
                "Conflicting account settings",
                "The given account settings conflict with the other existing accounts".to_owned(),
            ));
        }

        diesel::update(dsl::account.find(&self.id))
            .set(self)
            .execute(database_conn)?;

        Ok(())
    }

    /// List all accounts
    pub fn all(database_conn: &DatabaseConnection) -> ServiceResult<Vec<Account>> {
        use crate::model::schema::account::dsl;

        let results = dsl::account
            .order(dsl::name.asc())
            .load::<Account>(database_conn)?;

        Ok(results)
    }

    /// Get an account by the `id`
    pub fn get(database_conn: &DatabaseConnection, id: Uuid) -> ServiceResult<Account> {
        use crate::model::schema::account::dsl;

        let mut results = dsl::account
            .filter(dsl::id.eq(id))
            .load::<Account>(database_conn)?;

        results.pop().ok_or(ServiceError::NotFound)
    }

    /// Get an account by the `id`
    pub fn find_by_login(
        database_conn: &DatabaseConnection,
        login: &str,
    ) -> ServiceResult<Account> {
        use crate::model::schema::account::dsl;

        let mut results = match Uuid::parse_str(login) {
            Ok(uuid) => dsl::account
                .filter(dsl::id.eq(uuid))
                .load::<Account>(database_conn)?,
            Err(_) => dsl::account
                .filter(
                    dsl::mail
                        .eq(login)
                        .or(dsl::username.eq(login))
                        .or(dsl::account_number.eq(login)),
                )
                .load::<Account>(database_conn)?,
        };

        if results.len() > 1 {
            return Err(ServiceError::NotFound);
        }

        results.pop().ok_or(ServiceError::NotFound)
    }

    fn exist_conflicting_account(&self, database_conn: &DatabaseConnection) -> ServiceResult<bool> {
        use crate::model::schema::account::dsl;

        if !self.mail.is_empty() {
            let results = dsl::account
                .filter(dsl::id.ne(self.id).and(dsl::mail.eq(&self.mail)))
                .load::<Account>(database_conn)?;
            if !results.is_empty() {
                return Ok(false);
            }
        }

        if !self.username.is_empty() {
            let results = dsl::account
                .filter(dsl::id.ne(self.id).and(dsl::username.eq(&self.username)))
                .load::<Account>(database_conn)?;
            if !results.is_empty() {
                return Ok(false);
            }
        }

        if !self.account_number.is_empty() {
            let results = dsl::account
                .filter(
                    dsl::id
                        .ne(self.id)
                        .and(dsl::account_number.eq(&self.account_number)),
                )
                .load::<Account>(database_conn)?;
            if !results.is_empty() {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
