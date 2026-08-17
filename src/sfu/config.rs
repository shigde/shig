use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SfuConfig {
    /// Local address used to bind the RTC core UDP sockets.
    pub bind_ip: String,
    pub advertised_ip: String,
    /// Number of RTC cores. Zero selects the available OS parallelism.
    pub cores: usize,
    /// UDP port used by core 0. Following cores use consecutive ports.
    pub base_port: u16,
    /// Run every RTC core on its own Actix Arbiter / OS thread.
    pub dedicated_threads: bool,
    pub assignment: RtcAssignmentStrategy,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RtcAssignmentStrategy {
    RoundRobin,
    #[default]
    LeastLoaded,
}

impl Default for SfuConfig {
    fn default() -> Self {
        Self {
            bind_ip: "0.0.0.0".to_owned(),
            advertised_ip: String::new(),
            cores: 0,
            base_port: 50000,
            dedicated_threads: true,
            assignment: RtcAssignmentStrategy::LeastLoaded,
        }
    }
}
