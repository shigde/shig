use super::error::{RelayError, RelayResult};
use super::h264_diagnostics::H264Diagnostics;
use super::pipeline::{Frame, MediaPipeline, PipelineConfig, VideoCodec};
use super::video_group::VideoGroup;
use super::video_recovery::{VideoOutcome, VideoRecovery};
use super::RelayMediaStream;
use crate::sfu::media::Media;
use anyhow::{anyhow, bail, Context};
use bytes::Bytes;
use moq_mux::import::{AudioFormat, AudioInit, Track, VideoInit};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;

pub struct HangPublisher {
    media: crate::sfu::media::MediaService,
    origin: moq_net::origin::Producer,
    name: String,
    audio: Media,
    video: Media,
    audio_rx: broadcast::Receiver<std::sync::Arc<rtp::packet::Packet>>,
    video_rx: broadcast::Receiver<std::sync::Arc<rtp::packet::Packet>>,
    video_codec: VideoCodec,
    diagnostics_directory: Option<std::path::PathBuf>,
}

impl HangPublisher {
    pub fn new(
        media: crate::sfu::media::MediaService,
        origin: moq_net::origin::Producer,
        name: String,
        stream: RelayMediaStream,
        diagnostics_directory: Option<std::path::PathBuf>,
    ) -> RelayResult<Self> {
        let audio = stream
            .audio
            .ok_or_else(|| RelayError::NoInputTrack("audio".into()))?;
        let video = stream
            .video
            .ok_or_else(|| RelayError::NoInputTrack("video".into()))?;
        if !audio
            .capability
            .mime_type
            .eq_ignore_ascii_case("audio/opus")
        {
            return Err(RelayError::PublisherError(format!(
                "Unsupported audio codec: {}",
                audio.capability.mime_type
            )));
        }
        if audio.capability.clock_rate != 48_000 || !(1..=2).contains(&audio.capability.channels) {
            return Err(RelayError::PublisherError(
                "Opus requires a 48 kHz RTP clock and one or two channels".into(),
            ));
        }
        let video_codec = VideoCodec::from_mime(&video.capability.mime_type)
            .map_err(|err| RelayError::PublisherError(err.to_string()))?;
        let audio_rx = audio.relay_rtp_tx.subscribe();
        let video_rx = video.relay_rtp_tx.subscribe();
        Ok(Self {
            media,
            origin,
            name,
            audio,
            video,
            audio_rx,
            video_rx,
            video_codec,
            diagnostics_directory,
        })
    }

    pub async fn run(
        mut self,
        cancel: CancellationToken,
        ready: oneshot::Sender<RelayResult<()>>,
    ) -> anyhow::Result<()> {
        let config = PipelineConfig {
            video_codec: self.video_codec,
            video_payload_type: self.video.payload_type,
            video_clock_rate: self.video.capability.clock_rate,
            video_fmtp: self.video.capability.sdp_fmtp_line.clone(),
            audio_payload_type: self.audio.payload_type,
            audio_channels: self.audio.capability.channels,
        };
        let pipeline = tokio::task::spawn_blocking(move || MediaPipeline::start(config)).await?;
        let mut pipeline = match pipeline {
            Ok(pipeline) => pipeline,
            Err(err) => {
                let _ = ready.send(Err(RelayError::PublisherError(err.to_string())));
                return Err(err);
            }
        };
        let mut ready = Some(ready);
        let result = self.publish(&mut pipeline, &cancel, &mut ready).await;
        if let Some(ready) = ready.take() {
            let _ = ready.send(Err(RelayError::PublisherError(
                result
                    .as_ref()
                    .err()
                    .map(|err| format!("{err:#}"))
                    .unwrap_or_else(|| "Publisher cancelled during startup".into()),
            )));
        }
        // State transitions may join GStreamer threads; keep them off Tokio/Actix workers.
        let stopped = tokio::task::spawn_blocking(move || pipeline.stop()).await;
        result?;
        stopped??;
        Ok(())
    }

    async fn publish(
        &mut self,
        pipeline: &mut MediaPipeline,
        cancel: &CancellationToken,
        ready: &mut Option<oneshot::Sender<RelayResult<()>>>,
    ) -> anyhow::Result<()> {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let mut broadcast = self.origin.publish(&self.name, Default::default())?;
        let catalog = moq_mux::catalog::Producer::<()>::new(&mut broadcast, Default::default())?;
        // Keep the initial catalog pending until both track importers have been reserved.
        let reservation = catalog.reserve();
        let mut video = Track::video(
            broadcast.reserve_track("video")?,
            catalog.reserve(),
            VideoInit::new(self.video_codec.format(), Bytes::new()),
        )?;
        let opus_head =
            moq_mux::codec::opus::Config::new(48_000, self.audio.capability.channels as u32)
                .encode()?;
        let mut audio = Track::audio(
            broadcast.reserve_track("audio")?,
            catalog.reserve(),
            AudioInit::new(AudioFormat::Opus, opus_head),
        )?;
        drop(reservation);
        if let Err(error) = self.media.live_started(&self.name).await {
            log::error!("Recording registration failed for {}: {error:#}", self.name);
        }
        let _ = ready.take().unwrap().send(Ok(()));
        let mut video_group = VideoGroup::default();
        video_group.requested(Instant::now());
        let mut video_recovery = VideoRecovery::default();
        let mut keyframe_check = tokio::time::interval(Duration::from_millis(500));
        keyframe_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut diagnostics = if matches!(self.video_codec, VideoCodec::H264) {
            match self.diagnostics_directory.as_deref() {
                Some(directory) => H264Diagnostics::open(directory, &self.name).await,
                None => None,
            }
        } else {
            None
        };
        let result = async {
            tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                _ = self.video.request_keyframe(&self.video.peer_id) => {}
            }
            loop {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => break,
                    changed = pipeline.failure.changed() => {
                        changed.context("GStreamer error channel closed")?;
                        bail!("{}", pipeline.failure.borrow().as_deref().unwrap_or("GStreamer stopped"));
                    }
                    Some(()) = pipeline.keyframe_requests.recv() => {
                        video_group.reset();
                        if video_group.poll(Instant::now()) {
                            tokio::select! {
                                _ = cancel.cancelled() => break,
                                _ = self.video.request_keyframe(&self.video.peer_id) => {}
                            }
                        }
                    }
                    _ = keyframe_check.tick() => {
                        if video_group.poll(Instant::now()) {
                            tokio::select! {
                                _ = cancel.cancelled() => break,
                                _ = self.video.request_keyframe(&self.video.peer_id) => {}
                            }
                        }
                    }
                    frame = pipeline.frames.recv() => {
                        let frame = frame.context("GStreamer frame channel closed")?;
                        if frame.video {
                            // Capture the unmodified access unit before recovery or MoQ import.
                            if let Some(diagnostics) = diagnostics.as_mut() {
                                diagnostics.capture(&frame);
                            }
                            let flags = frame.sample.buffer().context("Missing video buffer")?.flags();
                            let size = frame.sample.buffer().unwrap().size();
                            let keyframe = !flags.contains(gstreamer::BufferFlags::DELTA_UNIT);
                            let now = Instant::now();
                            let outcome = video_recovery.publish(&mut video, frame, &self.name,
                                video_group.frames(), video_group.bytes(), now).with_context(|| format!(
                                "relay video group: frames={}, payload_bytes={}", video_group.frames(), video_group.bytes()
                            ))?;
                            let request = match outcome {
                                VideoOutcome::Published => video_group.observe(keyframe, size, now),
                                VideoOutcome::Dropped => continue,
                                VideoOutcome::Resync => {
                                    video_group.reset();
                                    video_group.poll(now)
                                }
                            };
                            if request {
                                tokio::select! {
                                    _ = cancel.cancelled() => break,
                                    _ = self.video.request_keyframe(&self.video.peer_id) => {}
                                }
                            }
                        }
                        else {
                            publish_frame(&mut audio, frame).context("relay audio frame")?;
                            audio.cut(None).context("close relay audio group")?;
                        }
                    }
                    packet = self.audio_rx.recv() => {
                        MediaPipeline::push(&pipeline.audio, packet.context("Relay audio RTP input lost")?.as_ref())?;
                    }
                    packet = self.video_rx.recv() => {
                        let packet = packet.context("Relay video RTP input lost")?;
                        if let Some(diagnostics) = diagnostics.as_mut() {
                            diagnostics.capture_rtp(&packet);
                        }
                        MediaPipeline::push(&pipeline.video, packet.as_ref())?;
                    }
                }
            }
            Ok(())
        }.await;
        // Finalize the recording while the publisher still owns both tracks.
        self.media.live_stopped(&self.name).await;
        if let Some(diagnostics) = diagnostics {
            diagnostics.finish().await;
        }
        let video_finished = video.finish().context("finish relay video track");
        let audio_finished = audio.finish().context("finish relay audio track");
        // A failed group can also make finish fail. Preserve the original write
        // error, including its track and group counters, after cleaning up both.
        result?;
        video_finished?;
        audio_finished?;
        Ok(())
    }
}

pub(super) fn publish_frame(track: &mut Track, frame: Frame) -> anyhow::Result<()> {
    let buffer = frame
        .sample
        .buffer_owned()
        .context("GStreamer returned no frame buffer")?;
    let pts = buffer
        .pts()
        .context("GStreamer returned no presentation timestamp")?;
    let timestamp = moq_net::Timestamp::from_micros(pts.useconds())?;
    let mapped = buffer
        .into_mapped_buffer_readable()
        .map_err(|_| anyhow!("Could not map GStreamer frame"))?;
    // Bytes owns the mapped GStreamer buffer until MoQ releases the frame.
    track.decode(Bytes::from_owner(mapped), Some(timestamp))?;
    Ok(())
}
