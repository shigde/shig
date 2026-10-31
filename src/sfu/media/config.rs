use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Optional storage override. Otherwise use files.htdocs/recordings.
    pub directory: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self { directory: None }
    }
}
