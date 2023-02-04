//! Simple single-file migration format
//!
//! This module implements a simple migration source for sqlx that loads all migrations from a single file,
//! which can be embedded into the binary.
//!
//! Every migration is marked by a line comment like  this:
//! ```
//! --##1 initial schema
//! ```
//! The comment specifies the version (1) and description (initial schema).
//! Each following migration should increase the version by one.
use std::{pin::Pin, future::Future, borrow::Cow};

use sqlx::{migrate::{MigrationSource, Migration, MigrationType}, error::BoxDynError};

#[derive(Debug)]
pub struct MigrationScript<'s> { data: &'s str }

impl<'s> MigrationSource<'s> for MigrationScript<'s> {
    fn resolve(self) -> Pin<Box<dyn Future<Output = Result<Vec<Migration>, BoxDynError>> + Send + 's>> {
        Box::pin(async move {
            let mut result = Vec::new();

            for line in self.data.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                if line.starts_with("--##") {
                    let version_end = line.find(' ').unwrap_or(line.len());
                    let version_str = &line[4..version_end];
                    let description_str = &line[version_end..];
                    let version = match version_str.parse() {
                        Ok(v) => v,
                        Err(e) => Err(format!("cannot parse version of migration as int, got string '{}', error: {}", version_str, e))?,
                    };
                    result.push(Migration::new(version, Cow::Owned(description_str.to_string()), MigrationType::Simple, Cow::Owned(String::new())));
                    continue;
                }

                let migration = match result.last_mut() {
                    Some(v) => v,
                    None => {
                        // allow comments at beginning of file
                        if line.starts_with("--") {
                            continue
                        }
                        Err(format!("migration script does not start with migration header, got: {}", line))?
                    }
                };
                migration.sql.to_mut().push_str(&line);
                migration.sql.to_mut().push('\n');
            }

            Ok(result)
        })
    }
}

pub fn postgresql_migrations() -> MigrationScript<'static> {
    MigrationScript { data: include_str!("./sql/migrations.pg.sql")}
}
