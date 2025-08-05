pub mod uploader;
pub mod error;

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct FilesConfig {
    pub htdocs: String,
}
