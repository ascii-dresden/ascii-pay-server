use diesel::prelude::*;

use crate::model::{DbConnection, Money, ServiceError, ServiceResult};

/// Get an account by the `id`
pub fn get_total_balance(conn: &DbConnection) -> ServiceResult<Money> {
    use crate::model::schema::account::dsl;

    let result: Option<i64> = dsl::account
        .select(diesel::dsl::sum(dsl::credit))
        .first(conn)?;

    result.map(|v| v as Money).ok_or(ServiceError::NotFound)
}
