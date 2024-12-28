pub mod error;

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
