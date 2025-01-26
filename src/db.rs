pub mod actors;
pub mod channels;
pub mod error;
pub mod fixtures;
pub mod instances;
pub mod schema;
pub mod user_roles;
pub mod users;
pub mod verification_tokens;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use serde::Deserialize;
use std::error::Error;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Deserialize)]
pub struct DbConfig {
    pub name: String,
}

pub fn build_pool(db_name: String) -> Result<DbPool, PoolError> {
    let manager = ConnectionManager::<SqliteConnection>::new(db_name);
    let pool = Pool::builder().build(manager)?;
    Ok(pool)
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
type DB = diesel::sqlite::Sqlite;

pub fn run_migrations(
    connection: &mut impl MigrationHarness<DB>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}
