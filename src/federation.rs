pub mod error;

use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct FederationConfig {
    pub enable: bool,
    pub domain: String,
    pub instance: String,
    pub token: String,
    pub tls: bool,
}
