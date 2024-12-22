pub mod schema;
pub mod actors;
pub mod instances;
pub mod users;
pub mod user_roles;

use std::error::Error;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool,  PoolError};
use diesel::result;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use serde_derive::Deserialize;
use crate::server;

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

pub fn run_migrations(connection: &mut impl MigrationHarness<DB>) -> Result<(), Box<dyn Error + Sync + Send>> {
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}
