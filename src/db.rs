pub mod schema;
pub mod actors;
pub mod instances;
pub mod users;
pub mod user_roles;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool,  PoolError};
use serde_derive::Deserialize;

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
