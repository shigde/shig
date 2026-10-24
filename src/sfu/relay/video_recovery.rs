use super::pipeline::Frame;
use super::publisher::publish_frame;
use anyhow::{Context, Result};
use gstreamer::BufferFlags;
use moq_mux::import::Track;
use std::time::{Duration, Instant};

pub(super) enum VideoOutcome {
    Published,
    Dropped,
    Resync,
}

/// Recover like moq-gst's media pad without changing timestamps or transport.
pub(super) struct VideoRecovery {
    waiting: bool,
    last_pts: Option<u64>,
    group_pts: Option<u64>,
    last_warning: Option<Instant>,
    last_recovery_log: Option<Instant>,
    rewinds: u64,
    dropped: u64,
}

impl Default for VideoRecovery {
    fn default() -> Self {
        Self {
            waiting: true,
            last_pts: None,
            group_pts: None,
            last_warning: None,
            last_recovery_log: None,
            rewinds: 0,
            dropped: 0,
        }
    }
}

impl VideoRecovery {
    pub fn publish(
        &mut self,
        track: &mut Track,
        frame: Frame,
        stream: &str,
        group_frames: usize,
        group_bytes: u64,
        now: Instant,
    ) -> Result<VideoOutcome> {
        let buffer = frame.sample.buffer().context("Missing video buffer")?;
        let flags = buffer.flags();
        let pts = buffer.pts().context("Missing video PTS")?.useconds();
        let keyframe = !flags.contains(BufferFlags::DELTA_UNIT);
        let broken = flags.intersects(BufferFlags::DISCONT | BufferFlags::CORRUPTED);
        if broken {
            track
                .discontinuity()
                .context("mark relay video discontinuity")?;
            self.waiting = true;
        }
        if flags.contains(BufferFlags::CORRUPTED) || (self.waiting && !keyframe) {
            self.dropped += 1;
            return Ok(if broken {
                VideoOutcome::Resync
            } else {
                VideoOutcome::Dropped
            });
        }
        match publish_frame(track, frame) {
            Ok(()) => {
                if self.waiting
                    && self.rewinds > 0
                    && self
                        .last_recovery_log
                        .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(5))
                {
                    log::info!(
                        "relay video recovered, stream_id={}, pts_us={}, rewinds={}, dropped={}",
                        stream,
                        pts,
                        self.rewinds,
                        self.dropped
                    );
                    self.last_recovery_log = Some(now);
                }
                self.waiting = false;
                self.last_pts = Some(pts);
                if keyframe {
                    self.group_pts = Some(pts);
                }
                Ok(VideoOutcome::Published)
            }
            Err(error) => match error.downcast_ref::<moq_mux::Error>() {
                Some(moq_mux::Error::TimestampRewind(_)) => {
                    // A rejected delta breaks its dependants even if the group stayed open.
                    track.cut(None).context("close rewound relay video group")?;
                    self.waiting = true;
                    self.rewinds += 1;
                    self.dropped += 1;
                    if self
                        .last_warning
                        .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(5))
                    {
                        log::warn!("relay video timestamp rewind; waiting for keyframe, stream_id={}, pts_us={}, last_accepted_pts_us={:?}, observed_group_start_pts_us={:?}, keyframe={}, flags={:?}, group_frames={}, group_bytes={}, rewinds={}, dropped={}",
                            stream, pts, self.last_pts, self.group_pts, keyframe, flags,
                            group_frames, group_bytes, self.rewinds, self.dropped);
                        self.last_warning = Some(now);
                    }
                    Ok(VideoOutcome::Resync)
                }
                Some(moq_mux::Error::MissingKeyframe(_)) => {
                    // Header-only buffers can decode successfully without opening a group.
                    self.waiting = true;
                    self.dropped += 1;
                    Ok(VideoOutcome::Resync)
                }
                _ => Err(error),
            },
        }
    }
}
