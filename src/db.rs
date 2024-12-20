mod actors;
mod instances;
mod schema;
mod users;
mod user_roles;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool,  PoolError};
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct DbConfig {
    pub name: String,
}

pub fn build_pool(db_name: String) -> Result<Pool<ConnectionManager<SqliteConnection>>, PoolError> {
    let manager = ConnectionManager::<SqliteConnection>::new(db_name);
    let pool = Pool::builder().build(manager)?;
    Ok(pool)
}
