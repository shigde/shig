pub mod error;

use std::error::Error;
use crate::db::instances::create::upsert_new_instance;
use crate::federation::error::{FederationError, FederationResult};
use diesel::SqliteConnection;
use serde::__private::de::IdentifierDeserializer;
use serde_derive::Deserialize;

#[derive(Deserialize, Clone)]
pub struct FederationConfig {
    pub enable: bool,
    pub domain: String,
    pub instance: String,
    pub token: String,
    pub tls: bool,
}

pub fn create_server_instance(
    conn: &mut SqliteConnection,
    cfg: FederationConfig,
) -> FederationResult<()> {
    if cfg.enable {
        upsert_new_instance(conn, cfg.instance.as_str(), cfg.domain.as_str(), cfg.tls)
            .map_err(|e| -> String { format!("upsert new instance: {}", e) })?;
        Ok(())
    } else {
        Ok(())
    }
}
