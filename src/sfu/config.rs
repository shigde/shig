use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SfuConfig {
    pub advertised_ip: String,
    pub port_min: u16,
    pub port_max: u16,
}

impl Default for SfuConfig {
    fn default() -> Self {
        Self {
            advertised_ip: String::new(),
            port_min: 50000,
            port_max: 51000,
        }
    }
}
