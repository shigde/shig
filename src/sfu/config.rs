use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct H264DiagnosticsConfig {
    pub enabled: bool,
    pub directory: PathBuf,
}

impl Default for H264DiagnosticsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            directory: PathBuf::from("debug/hls"),
        }
    }
}

impl H264DiagnosticsConfig {
    /// Resolve the sole diagnostic output directory during server initialization.
    pub fn initialize(&self) -> anyhow::Result<Option<PathBuf>> {
        if !self.enabled {
            return Ok(None);
        }
        anyhow::ensure!(
            !self.directory.as_os_str().is_empty(),
            "H264 diagnostic directory is empty"
        );
        std::fs::create_dir_all(&self.directory)?;
        Ok(Some(std::fs::canonicalize(&self.directory)?))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SfuConfig {
    pub advertised_ip: String,
    pub port_min: u16,
    pub port_max: u16,
    pub host_grace_seconds: u64,
    pub h264_diagnostics: H264DiagnosticsConfig,
}

impl Default for SfuConfig {
    fn default() -> Self {
        Self {
            advertised_ip: String::new(),
            port_min: 50000,
            port_max: 51000,
            host_grace_seconds: 120,
            h264_diagnostics: H264DiagnosticsConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_are_disabled_for_existing_configs_and_do_not_create_directories() {
        let config: SfuConfig = toml::from_str("port_min = 50000").unwrap();
        assert!(!config.h264_diagnostics.enabled);
        let directory =
            std::env::temp_dir().join(format!("shig-disabled-{}", uuid::Uuid::new_v4()));
        let diagnostics = H264DiagnosticsConfig {
            enabled: false,
            directory: directory.clone(),
        };
        assert!(diagnostics.initialize().unwrap().is_none());
        assert!(!directory.exists());
    }

    #[test]
    fn diagnostic_directory_is_initialized_and_resolved_from_config() {
        let directory =
            std::env::temp_dir().join(format!("shig-diagnostics-{}", uuid::Uuid::new_v4()));
        let diagnostics = H264DiagnosticsConfig {
            enabled: true,
            directory: directory.clone(),
        };
        let resolved = diagnostics.initialize().unwrap().unwrap();
        assert_eq!(resolved, std::fs::canonicalize(&directory).unwrap());
        assert!(resolved.is_dir());
        std::fs::remove_dir(directory).unwrap();
        assert!(H264DiagnosticsConfig {
            enabled: true,
            directory: PathBuf::new()
        }
        .initialize()
        .is_err());
    }
}
