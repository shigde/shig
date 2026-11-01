//! Recording control boundary. Archive writing and reading are not connected yet.
use super::{config::Config, recording::Recording};
use anyhow::bail;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct MediaService {
    root: PathBuf,
    live: Arc<Mutex<HashSet<String>>>,
    recording_operations:
        Arc<Mutex<std::collections::HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}
impl Default for MediaService {
    fn default() -> Self {
        Self::new(Config::default())
    }
}
impl MediaService {
    pub fn new(config: Config) -> Self {
        Self::with_htdocs(config, "htdocs")
    }
    pub fn with_htdocs(config: Config, htdocs: &str) -> Self {
        Self {
            root: config
                .directory
                .unwrap_or_else(|| Path::new(htdocs).join("recordings")),
            live: Default::default(),
            recording_operations: Default::default(),
        }
    }
    pub fn storage_directory(&self) -> &Path {
        &self.root
    }
    pub async fn recording_operation(&self, stream: &str) -> tokio::sync::OwnedMutexGuard<()> {
        let operation = self
            .recording_operations
            .lock()
            .unwrap()
            .entry(stream.into())
            .or_default()
            .clone();
        operation.lock_owned().await
    }
    pub async fn status(&self, stream: &str) -> anyhow::Result<Recording> {
        uuid::Uuid::parse_str(stream)?;
        Ok(Recording {
            stream_uuid: stream.into(),
            status: "empty".into(),
            published: false,
            duration_ms: 0,
            error: None,
        })
    }
    pub async fn live_started(&self, stream: &str) -> anyhow::Result<()> {
        uuid::Uuid::parse_str(stream)?;
        self.live.lock().unwrap().insert(stream.into());
        Ok(())
    }
    pub async fn live_stopped(&self, stream: &str) {
        self.live.lock().unwrap().remove(stream);
        if let Err(error) = self.stop(stream).await {
            log::error!("Stop recording {stream}: {error:#}");
        }
    }
    pub async fn start(&self, stream: &str) -> anyhow::Result<Recording> {
        uuid::Uuid::parse_str(stream)?;
        if !self.live.lock().unwrap().contains(stream) {
            bail!("Stream is not live");
        }
        // Do not report a running recording until the archive writer is connected.
        bail!("Archive recording is not connected yet");
    }
    pub async fn stop(&self, stream: &str) -> anyhow::Result<Recording> {
        // No writer is active after removing the custom recorder.
        self.status(stream).await
    }
    pub async fn publish(&self, stream: &str, published: bool) -> anyhow::Result<Recording> {
        let recording = self.status(stream).await?;
        if published {
            bail!("No archive recording is available");
        }
        Ok(recording)
    }
}

#[cfg(test)]
#[path = "media_service_test.rs"]
mod tests;
