pub mod active_users;
pub mod actor_images;
pub mod actors;
pub mod channels;
pub mod error;
pub mod fixtures;
pub mod instances;
pub mod lobbies;
pub mod schema;
pub mod schema_views;
pub mod stream_meta_data;
pub mod stream_participants;
pub mod stream_previews;
pub mod stream_thumbnails;
pub mod streams;
pub mod user_roles;
pub mod users;
pub mod verification_tokens;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use serde::Deserialize;
use std::error::Error;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Deserialize, Clone, Debug)]
pub struct DbConfig {
    pub connection: String,
    #[allow(dead_code)]
    pub pool_size: i32,
}

pub fn build_pool(cfg: DbConfig) -> Result<DbPool, PoolError> {
    let manager = ConnectionManager::<PgConnection>::new(cfg.connection);
    let pool = Pool::builder().build(manager)?;
    Ok(pool)
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
type DB = diesel::pg::Pg;

pub fn run_migrations(
    connection: &mut impl MigrationHarness<DB>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}
