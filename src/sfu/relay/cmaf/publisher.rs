use crate::sfu::relay::cmaf::cmaf_track_writer::{write_cmaf_track, write_fragment};
use crate::sfu::relay::cmaf::prepared_cmaf_track::PreparedCmafTrack;
use crate::sfu::relay::error::{RelayError, RelayResult};
use bytes::Bytes;
use hang::moq_lite;
use std::collections::BTreeMap;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub struct HangAvPublisher {
    origin: moq_lite::OriginProducer,
    name: String,
    video_rx: mpsc::Receiver<Bytes>,
    audio_rx: mpsc::Receiver<Bytes>,
}

impl HangAvPublisher {
    pub fn new(
        origin: moq_lite::OriginProducer,
        name: String,
        video_rx: mpsc::Receiver<Bytes>,
        audio_rx: mpsc::Receiver<Bytes>,
    ) -> Self {
        Self {
            origin,
            name,
            video_rx,
            audio_rx,
        }
    }

    pub async fn run(self, cancel: CancellationToken) -> RelayResult<()> {
        let mut video = PreparedCmafTrack::build(self.video_rx, cancel.clone())
            .await
            .map_err(|e| {
                RelayError::CmafPublisher(format!("prepare video CMAF track: {}", e.to_string()))
            })?;

        let mut audio = PreparedCmafTrack::build(self.audio_rx, cancel.clone())
            .await
            .map_err(|e| {
                RelayError::CmafPublisher(format!("prepare audio CMAF track: {}", e.to_string()))
            })?;

        let mut broadcast = moq_lite::Broadcast::produce();

        let video_track_info = moq_lite::Track {
            name: "video".to_string(),
            priority: 1,
        };

        let audio_track_info = moq_lite::Track {
            name: "audio".to_string(),
            priority: 0,
        };

        publish_catalog(&mut broadcast, &video_track_info, &audio_track_info).map_err(|e| {
            RelayError::CmafPublisher(format!("publish catalog: {}", e.to_string()))
        })?;

        let mut video_track = broadcast.create_track(video_track_info).map_err(|e| {
            RelayError::CmafPublisher(format!("create video CMAF track: {}", e.to_string()))
        })?;
        let mut audio_track = broadcast.create_track(audio_track_info).map_err(|e| {
            RelayError::CmafPublisher(format!("create audio CMAF track: {}", e.to_string()))
        })?;

        write_fragment(&mut video_track, video.init.clone())?;
        write_fragment(&mut audio_track, audio.init.clone())?;

        let name = self.name.clone();
        self.origin.publish_broadcast(name, broadcast.consume());

        let video_cancel = cancel.clone();
        let audio_cancel = cancel.clone();

        let video_task = tokio::spawn(async move {
            write_cmaf_track("video", video_track, &mut video, video_cancel).await
        });

        let audio_task = tokio::spawn(async move {
            write_cmaf_track("audio", audio_track, &mut audio, audio_cancel).await
        });

        tokio::select! {
            result = video_task => {
                let inner = result.map_err(|e| {
                    RelayError::CmafPublisher(format!("join error: {}", e))
                })?;

                inner?;

                cancel.cancel();
            }

            result = audio_task => {
                let inner = result.map_err(|e| {
                    RelayError::CmafPublisher(format!("join error: {}", e))
                })?;

                inner?;

                cancel.cancel();
            }

            _ = cancel.cancelled() => {}
        }

        Ok(())
    }
}

fn publish_catalog(
    broadcast: &mut moq_lite::BroadcastProducer,
    video_track: &moq_lite::Track,
    audio_track: &moq_lite::Track,
) -> RelayResult<()> {
    let video_config = hang::catalog::VideoConfig {
        codec: hang::catalog::H264 {
            profile: 0x4D,
            constraints: 0,
            level: 0x28,
            inline: false,
        }
        .into(),
        description: None,
        coded_width: Some(1920),
        coded_height: Some(1080),
        bitrate: Some(5_000_000),
        framerate: Some(30.0),
        display_ratio_width: None,
        display_ratio_height: None,
        optimize_for_latency: Some(true),
        container: hang::catalog::Container::Cmaf {
            timescale: 0,
            track_id: 0,
        },
        jitter: None,
    };

    let audio_config = hang::catalog::AudioConfig {
        codec: hang::catalog::AAC {
            profile: 2, // AAC-LC
        }
        .into(),
        sample_rate: 48_000,
        channel_count: 2,
        bitrate: Some(128_000),
        description: None,
        container: hang::catalog::Container::Cmaf {
            timescale: 0,
            track_id: 0,
        },
        jitter: None,
    };

    let mut video_renditions = BTreeMap::new();
    video_renditions.insert(video_track.name.clone(), video_config);

    let mut audio_renditions = BTreeMap::new();
    audio_renditions.insert(audio_track.name.clone(), audio_config);

    let catalog = hang::catalog::Catalog {
        video: hang::catalog::Video {
            renditions: video_renditions,
            display: None,
            rotation: None,
            flip: None,
        },
        audio: hang::catalog::Audio {
            renditions: audio_renditions,
        },
        ..Default::default()
    };

    let mut catalog_track = broadcast
        .create_track(hang::Catalog::default_track())
        .map_err(|e| RelayError::CmafPublisher(format!("create catalog track: {}", e)))?;

    let mut group = catalog_track
        .append_group()
        .map_err(|e| RelayError::CmafPublisher(format!("append group: {}", e)))?;

    group
        .write_frame(
            catalog
                .to_string()
                .map_err(|e| RelayError::CmafPublisher(format!("serialize catalog: {}", e)))?,
        )
        .map_err(|e| RelayError::CmafPublisher(format!("write frame: {}", e)))?;

    group
        .finish()
        .map_err(|e| RelayError::CmafPublisher(format!("finish group: {}", e)))?;

    Ok(())
}
