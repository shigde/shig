pub mod uploader;
pub mod error;

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct FilesConfig {
    pub htdocs: String,
}
