use crate::sfu::relay::cmaf::box_payload::extract_box_payload;
use crate::sfu::relay::cmaf::cmaf_track_writer::{write_cmaf_track, write_fragment};
use crate::sfu::relay::cmaf::prepared_cmaf_track::PreparedCmafTrack;
use crate::sfu::relay::error::{RelayError, RelayResult};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bytes::Bytes;
use hang::moq_lite;
use moq_lite::Error;
use std::collections::BTreeMap;
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

pub struct HangAvPublisher {
    origin: moq_lite::OriginProducer,
    name: String,
    video_rx: mpsc::Receiver<Bytes>,
    audio_rx: mpsc::Receiver<Bytes>,
    publisher_ready_tx: watch::Sender<bool>,
}

impl HangAvPublisher {
    pub fn new(
        origin: moq_lite::OriginProducer,
        name: String,
        video_rx: mpsc::Receiver<Bytes>,
        audio_rx: mpsc::Receiver<Bytes>,
        publisher_ready_tx: watch::Sender<bool>,
    ) -> Self {
        Self {
            origin,
            name,
            video_rx,
            audio_rx,
            publisher_ready_tx,
        }
    }

    pub async fn run(self, cancel: CancellationToken, stopped: CancellationToken) -> RelayResult<()> {
        log::info!("hang av publisher started");

        let video_rx = self.video_rx;
        let audio_rx = self.audio_rx;

        let video_cancel = cancel.clone();
        let audio_cancel = cancel.clone();

        log::info!(
            "video and audio CMAF tracks prepared, name={}",
            self.name.clone()
        );

        let mut broadcast = moq_lite::Broadcast::produce();

        let video_track_info = moq_lite::Track {
            name: "video0".to_string(),
            priority: 1,
        };

        let audio_track_info = moq_lite::Track {
            name: "audio1".to_string(),
            priority: 0,
        };

        // The Publisher is ready because both Receivers now exist
        self.publisher_ready_tx
            .send(true)
            .map_err(|e| RelayError::CmafPublisher(format!("send publisher ready: {}", e)))?;

        let mut video_track = broadcast
            .create_track(video_track_info.clone())
            .map_err(|e| RelayError::CmafPublisher(format!("create cmf video track: {}", e)))?;
        let mut audio_track = broadcast
            .create_track(audio_track_info.clone())
            .map_err(|e| RelayError::CmafPublisher(format!("create cmf audio track: {}", e)))?;

        let mut video = PreparedCmafTrack::build(
            video_track.info.name.clone(),
            video_rx,
            video_cancel.clone(),
        )
        .await?;

        let mut audio = PreparedCmafTrack::build(
            audio_track.info.name.clone(),
            audio_rx,
            audio_cancel.clone(),
        )
        .await?;

        log::info!(
            "video init for publisher  name={} contains avcC= {}",
            self.name.clone(),
            video.init.windows(4).any(|w| w == b"avcC"),
        );

        let video_avcc = extract_box_payload(&video.init, b"avcC")
            .ok_or_else(|| RelayError::CmafPublisher("missing avcC".into()))?;

        let audio_esds = extract_box_payload(&audio.init, b"esds")
            .ok_or_else(|| RelayError::CmafPublisher("missing esds".into()))?;

        // let avcc = extract_avcc(&video.init)
        //     .ok_or_else(|| RelayError::CmafPublisher("missing avcC".into()))?;

        let _catalog_track = publish_catalog(
            &mut broadcast,
            &video_track_info,
            &audio_track_info,
            video.init.clone(),
            audio.init.clone(),
            video_avcc,
            audio_esds,
        )
        .map_err(|e| RelayError::CmafPublisher(format!("publish catalog: {}", e.to_string())))?;

        write_fragment(&mut video_track, video.init.clone())?;
        write_fragment(&mut audio_track, audio.init.clone())?;

        let video_task = tokio::spawn(async move {
            write_cmaf_track("video0".to_string(), video_track, &mut video, video_cancel).await
        });

        let audio_task = tokio::spawn(async move {
            write_cmaf_track("audio1".to_string(), audio_track, &mut audio, audio_cancel).await
        });

        let name = self.name.clone();
        self.origin.publish_broadcast(name.clone(), broadcast.consume());

        tokio::select! {
            result = video_task => {
                let inner = result.map_err(|e| {
                    log::error!("video task error: {}", e);
                    RelayError::CmafPublisher(format!("join error: {}", e))
                })?;

                inner?;

                stopped.cancel();
            }

            result = audio_task => {
                let inner = result.map_err(|e| {
                    log::error!("audio task error: {}", e);
                    RelayError::CmafPublisher(format!("join error: {}", e))
                })?;

                inner?;

                stopped.cancel();
            }

            _ = cancel.cancelled() => {
                log::info!("hang av publisher cancelled, name={}", name);
                if let Err(e) = broadcast.abort(Error::Cancel) {
                    log::error!("abort publisher: name={}, error={}", name, e);
                    stopped.cancel();
                    return Err(RelayError::CmafPublisher(format!("abort publisher: {}", e)));
                }

                log::info!("relay hang publisher aborted, name={}", name);
                stopped.cancel();
                return Ok(());
            }
        }

        Ok(())
    }
}

fn publish_catalog(
    broadcast: &mut moq_lite::BroadcastProducer,
    video_track: &moq_lite::Track,
    audio_track: &moq_lite::Track,
    video_init: Bytes,
    audio_init: Bytes,
    video_avcc: Bytes,
    #[allow(unused)] audio_esds: Bytes,
) -> RelayResult<moq_lite::TrackProducer> {
    let video_config = hang::catalog::VideoConfig {
        codec: hang::catalog::H264 {
            profile: 0x42,
            constraints: 0xC0,
            level: 0x1E,
            inline: false,
        }
        .into(),
        description: None,
        coded_width: Some(640),
        coded_height: Some(360),
        bitrate: Some(5_000_000),
        framerate: Some(30.0),
        display_ratio_width: None,
        display_ratio_height: None,
        optimize_for_latency: Some(true),
        container: hang::catalog::Container::Cmaf {
            timescale: 90_000,
            track_id: 1,
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
        container: hang::catalog::Container::Legacy {},
        jitter: None,
    };

    let mut video_renditions = BTreeMap::new();
    video_renditions.insert(video_track.name.clone(), video_config);

    let mut audio_renditions = BTreeMap::new();
    audio_renditions.insert(audio_track.name.clone(), audio_config);

    let _catalog = hang::catalog::Catalog {
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

    // let catalog_json = catalog
    //     .to_string()
    //     .map_err(|e| RelayError::CmafPublisher(format!("serialize catalog: {}", e)))?;

    let catalog_json = serde_json::json!({
        "video": {
            "renditions": {
                "video0": {
    // avc1.42c020 - H264 Baseline
    // avc1.4d0028 - H264 Main
    // avc1.640028 - H264 High
                    "codec": "avc1.42c020",
                    "container": {
                        "kind": "cmaf",
                        "init": STANDARD.encode(&video_init),
                    },
                    "description": hex::encode(&video_avcc),
                    "codedWidth": 640,
                    "codedHeight": 360,
                    "bitrate": 5_000_000,
                    "framerate": 30.0,
                    "optimizeForLatency": true,
                    //"jitter": 300,
                }
            }
        },

        "audio": {
            "renditions": {
                "audio1": {
                    // AAC-LC
                    "codec": "mp4a.40.2",
                    "container": {
                        "kind": "cmaf",
                        "init": STANDARD.encode(&audio_init),
                    },
                    "description": "1190",
                    //"description": hex::encode(&audio_esds),
                    "sampleRate": 48_000,
                    "numberOfChannels": 2,
                    "bitrate": 128_000,
                    //"jitter": 100
                }
            }
        }
    })
    .to_string();

    log::info!("MOQ catalog: {}", catalog_json);
    group
        .write_frame(catalog_json)
        .map_err(|e| RelayError::CmafPublisher(format!("write frame: {}", e)))?;

    // group
    //     .write_frame(
    //         catalog
    //             .to_string()
    //             .map_err(|e| RelayError::CmafPublisher(format!("serialize catalog: {}", e)))?,
    //     )
    //     .map_err(|e| RelayError::CmafPublisher(format!("write frame: {}", e)))?;

    group
        .finish()
        .map_err(|e| RelayError::CmafPublisher(format!("finish group: {}", e)))?;

    Ok(catalog_track)
}
