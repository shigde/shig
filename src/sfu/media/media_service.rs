//! Recording lifecycle; media processing and HLS persistence live in HlsWriter.
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
    recordings: Arc<Mutex<std::collections::HashMap<String, ActiveRecording>>>,
    recording_operations:
        Arc<Mutex<std::collections::HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

#[derive(Clone)]
struct ActiveRecording {
    video_uuid: String,
    stop: tokio_util::sync::CancellationToken,
    result: tokio::sync::watch::Receiver<Option<Result<u64, String>>>,
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
            recordings: Default::default(),
            recording_operations: Default::default(),
        }
    }
    pub fn storage_directory(&self) -> &Path {
        &self.root
    }
    pub async fn wait_recording_ready(&self, source: moq_mux::Source) -> anyhow::Result<()> {
        super::hls_writer::HlsWriter::prepare(source).await?;
        Ok(())
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
        let active = self.recordings.lock().unwrap().get(stream).cloned();
        let result = active.as_ref().and_then(|a| a.result.borrow().clone());
        let (status, duration_ms, error) = match result {
            Some(Ok(duration)) => ("ready", duration, None),
            Some(Err(error)) => ("failed", 0, Some(error)),
            None if active.is_some() => ("recording", 0, None),
            None => ("empty", 0, None),
        };
        Ok(Recording {
            stream_uuid: stream.into(),
            status: status.into(),
            published: false,
            duration_ms,
            error,
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
        bail!("Start recordings through the Recorded Video endpoint");
    }
    pub async fn start_recording(
        &self,
        stream: &str,
        video: &str,
        source: moq_mux::Source,
        pool: crate::db::DbPool,
    ) -> anyhow::Result<()> {
        uuid::Uuid::parse_str(stream)?;
        uuid::Uuid::parse_str(video)?;
        if !self.live.lock().unwrap().contains(stream) {
            bail!("Stream is not live");
        }
        if self
            .recordings
            .lock()
            .unwrap()
            .get(stream)
            .is_some_and(|a| a.result.borrow().is_none())
        {
            bail!("A recording is already running");
        }
        let writer = super::hls_writer::HlsWriter::open(source, self.root.join(video)).await?;
        let stop = tokio_util::sync::CancellationToken::new();
        let (finished, result) = tokio::sync::watch::channel(None);
        self.recordings.lock().unwrap().insert(
            stream.into(),
            ActiveRecording {
                video_uuid: video.to_owned(),
                stop: stop.clone(),
                result,
            },
        );
        let video = video.to_owned();
        tokio::spawn(async move {
            let outcome = writer.run(stop).await.map_err(|error| format!("{error:#}"));
            // Persist completion also when the source ends without a Stop HTTP request.
            let persisted = tokio::task::spawn_blocking(move || -> Result<u64, String> {
                let mut conn = pool.get().map_err(|e| e.to_string())?;
                match outcome {
                    Ok(duration) => {
                        crate::db::recorded_videos::recording_stopped(&mut conn, &video)
                            .map_err(|e| e.to_string())?;
                        crate::db::recorded_videos::recording_ready(&mut conn, &video, duration)
                            .map_err(|e| e.to_string())?;
                        Ok(duration)
                    }
                    Err(error) => {
                        crate::db::recorded_videos::recording_failed(&mut conn, &video, &error)
                            .map_err(|e| e.to_string())?;
                        Err(error)
                    }
                }
            })
            .await
            .unwrap_or_else(|error| Err(error.to_string()));
            if let Err(error) = &persisted {
                log::error!("HLS recording: {error}");
            }
            let _ = finished.send(Some(persisted));
        });
        Ok(())
    }
    pub async fn stop(&self, stream: &str) -> anyhow::Result<Recording> {
        let active = self.recordings.lock().unwrap().get(stream).cloned();
        if let Some(mut active) = active {
            active.stop.cancel();
            loop {
                let result = active.result.borrow().clone();
                if let Some(result) = result {
                    result.map_err(anyhow::Error::msg)?;
                    break;
                }
                active
                    .result
                    .changed()
                    .await
                    .map_err(|_| anyhow::anyhow!("Recording worker stopped unexpectedly"))?;
            }
        }
        self.status(stream).await
    }
    pub async fn delete_recording_files(&self, video: &str) -> anyhow::Result<()> {
        let id = uuid::Uuid::parse_str(video)?.to_string();
        if self
            .recordings
            .lock()
            .unwrap()
            .values()
            .any(|active| active.video_uuid == id && active.result.borrow().is_none())
        {
            bail!("Stop the recording before deleting it");
        }
        let directory = self.root.join(&id);
        match tokio::fs::symlink_metadata(&directory).await {
            Ok(metadata) if metadata.is_dir() => tokio::fs::remove_dir_all(directory).await?,
            Ok(_) => tokio::fs::remove_file(directory).await?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        self.recordings
            .lock()
            .unwrap()
            .retain(|_, active| active.video_uuid != id);
        Ok(())
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
