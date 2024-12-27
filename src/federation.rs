pub mod error;

use crate::db::instances::create::upsert_new_instance;
use crate::federation::error::{FederationResult};
use diesel::SqliteConnection;
use serde_derive::Deserialize;

#[allow(dead_code)]
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
