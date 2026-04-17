use crate::relay::mp4;
use crate::relay::stats::PublisherStats;
use crate::relay::track_state::{TrackState, TrackType};
use anyhow::anyhow;
use base64::Engine;
use bytes::Bytes;
use moq_lite::{BroadcastProducer, Group, GroupProducer, Track, TrackProducer};
use moq_mux::CatalogProducer;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Shared time origin across all tracks for synchronized rebasing.
/// When the first track's BDT arrives, we record (bdt, timescale) as the
/// wall-clock reference. All other tracks compute their time_base relative
/// to this same origin, ensuring audio and video timestamps stay in sync.
struct SharedTimeOrigin {
    /// Wall-clock time (seconds) of the first BDT seen across all tracks.
    origin_seconds: f64,
}

pub struct VideoStructure {
    pub segment_duration_ms: u64,
    pub fragments_per_segment: u32,
    pub fragment_duration_ms: f64,
    pub fps: f64,
    pub timescale: u32,
    pub default_sample_duration: Option<u32>,
}

pub struct Publisher {
    broadcast: BroadcastProducer,
    catalog: CatalogProducer,
    tracks: HashMap<String, TrackState>,
    init_segments: HashMap<String, Vec<u8>>,
    video_count: u32,
    audio_count: u32,
    /// CMSF SAP-type event timeline track (one shared across all media tracks).
    sap_track: Option<TrackProducer>,
    sap_group: Option<GroupProducer>,
    catalog_last_logged: Option<std::time::Instant>,
    /// Shared time origin for synchronized A/V timestamp rebasing.
    time_origin: Option<SharedTimeOrigin>,
    /// Target latency in ms for MSF catalog (default 2000).
    target_latency_ms: Option<u64>,
    /// When the first video track was registered (for audio-wait timeout).
    first_video_at: Option<std::time::Instant>,
    /// Expected track counts from --tracks flag (e.g. 3v1a).
    expected_video: Option<u32>,
    expected_audio: Option<u32>,
    /// Shared stats counters readable from the stats loop.
    pub stats: Arc<PublisherStats>,
}

impl Publisher {
    pub fn new(
        broadcast: BroadcastProducer,
        catalog: CatalogProducer,
        stats: Arc<PublisherStats>,
    ) -> Self {
        Self {
            broadcast,
            catalog,
            tracks: HashMap::new(),
            init_segments: HashMap::new(),
            video_count: 0,
            audio_count: 0,
            sap_track: None,
            sap_group: None,
            catalog_last_logged: None,
            time_origin: None,
            target_latency_ms: None,
            first_video_at: None,
            expected_video: None,
            expected_audio: None,
            stats,
        }
    }

    pub fn set_target_latency_ms(&mut self, ms: u64) {
        self.target_latency_ms = Some(ms);
    }

    pub fn set_expected_tracks(&mut self, video: u32, audio: u32) {
        self.expected_video = Some(video);
        self.expected_audio = Some(audio);
    }

    pub fn register_init(
        &mut self,
        handler_type: &str,
        init_data: &[u8],
    ) -> Result<String, anyhow::Error> {
        let track_type = match handler_type {
            "vide" => TrackType::Video,
            "soun" => TrackType::Audio,
            other => return Err(anyhow!("unknown handler type: {}", other)),
        };

        let codec_str = mp4::parse_codec_from_init(init_data).unwrap_or_else(|| {
            if track_type == TrackType::Video {
                "avc1".to_string()
            } else {
                "mp4a.40.2".to_string()
            }
        });
        let timescale = mp4::parse_timescale(init_data).unwrap_or(90000);
        let track_id = mp4::parse_track_id_from_init(init_data).unwrap_or(1);
        let default_sample_duration = mp4::extract_default_sample_duration(init_data)
            .filter(|&dur| {
                // Reject unreasonable values: if default_sample_duration >= timescale,
                // it means ≥1 second per sample (e.g. Ateme trex placeholder of 30000
                // at timescale=30000). Injecting this would corrupt timestamps.
                if dur >= timescale {
                    log::info!("Ignoring trex default_sample_duration={} (>= timescale={}), likely placeholder", dur, timescale);
                    false
                } else {
                    true
                }
            });

        let track_name = match track_type {
            TrackType::Video => {
                let description = mp4::extract_avcc_bytes(init_data);
                let (width, height) = mp4::extract_video_dimensions(init_data).unwrap_or((0, 0));

                let video_codec: hang::catalog::VideoCodec = codec_str
                    .parse()
                    .unwrap_or(hang::catalog::VideoCodec::Unknown(codec_str.clone()));

                let config = hang::catalog::VideoConfig {
                    codec: video_codec,
                    description: description.map(Bytes::from),
                    coded_width: if width > 0 { Some(width) } else { None },
                    coded_height: if height > 0 { Some(height) } else { None },
                    display_ratio_width: None,
                    display_ratio_height: None,
                    bitrate: None,
                    framerate: None,
                    optimize_for_latency: Some(true),
                    container: hang::catalog::Container::Cmaf {
                        timescale: timescale as u64,
                        track_id,
                    },
                    jitter: None,
                };

                let mut cat = self.catalog.lock();
                let track_info: Track = cat.video.create_track("m4s", config);
                let name = track_info.name.clone();

                let track_producer = self
                    .broadcast
                    .create_track(track_info)
                    .map_err(|e| anyhow!("failed to create video track: {}", e))?;

                self.video_count += 1;
                if self.first_video_at.is_none() {
                    self.first_video_at = Some(std::time::Instant::now());
                }
                self.init_segments.insert(name.clone(), init_data.to_vec());
                log::info!("Registered video track '{}' codec={} {}x{} timescale={} default_sample_duration={:?}",
                    name, codec_str, width, height, timescale, default_sample_duration);

                // Update shared stats
                self.stats.video_width.store(width, Ordering::Relaxed);
                self.stats.video_height.store(height, Ordering::Relaxed);
                *self.stats.video_codec.lock().unwrap() = codec_str.clone();

                self.tracks.insert(
                    name.clone(),
                    TrackState {
                        track: track_producer,
                        timescale,
                        default_sample_duration,
                        group: None,
                        time_base: None,
                        track_type: TrackType::Video,
                        new_segment: false,
                        last_bdt: None,
                        last_frag_duration: None,
                    },
                );

                // Create SAP event timeline track on first video track
                if self.sap_track.is_none() {
                    let sap_info = Track::new("sap-timeline");
                    match self.broadcast.create_track(sap_info) {
                        Ok(producer) => {
                            log::info!("Created CMSF SAP event timeline track");
                            self.sap_track = Some(producer);
                        }
                        Err(e) => log::info!("Failed to create SAP timeline track: {}", e),
                    }
                }

                name
            }
            TrackType::Audio => {
                let description = mp4::extract_esds_bytes(init_data);
                let sample_rate = mp4::extract_audio_sample_rate(init_data).unwrap_or(48000);
                let channels = mp4::extract_audio_channels(init_data).unwrap_or(2);

                let audio_codec: hang::catalog::AudioCodec = codec_str
                    .parse()
                    .unwrap_or(hang::catalog::AudioCodec::Unknown(codec_str.clone()));

                let config = hang::catalog::AudioConfig {
                    codec: audio_codec,
                    description: description.map(Bytes::from),
                    sample_rate,
                    channel_count: channels as u32,
                    bitrate: None,
                    container: hang::catalog::Container::Cmaf {
                        timescale: timescale as u64,
                        track_id,
                    },
                    jitter: None,
                };

                let mut cat = self.catalog.lock();
                let track_info: Track = cat.audio.create_track("m4s", config);
                let name = track_info.name.clone();

                let track_producer = self
                    .broadcast
                    .create_track(track_info)
                    .map_err(|e| anyhow!("failed to create audio track: {}", e))?;

                self.audio_count += 1;
                self.init_segments.insert(name.clone(), init_data.to_vec());
                log::info!("Registered audio track '{}' codec={} sr={} ch={} timescale={} default_sample_duration={:?}",
                    name, codec_str, sample_rate, channels, timescale, default_sample_duration);

                // Update shared stats
                *self.stats.audio_codec.lock().unwrap() = codec_str.clone();

                self.tracks.insert(
                    name.clone(),
                    TrackState {
                        track: track_producer,
                        timescale,
                        default_sample_duration,
                        group: None,
                        time_base: None,
                        track_type: TrackType::Audio,
                        new_segment: false,
                        last_bdt: None,
                        last_frag_duration: None,
                    },
                );

                name
            }
        };

        self.stats
            .track_count
            .store(self.tracks.len() as u32, Ordering::Relaxed);

        // Only publish catalog once we have all expected tracks.
        if self.has_complete_catalog() {
            self.publish_msf_catalog();
        } else {
            let expected = match (self.expected_video, self.expected_audio) {
                (Some(ev), Some(ea)) => format!("expected {}v{}a", ev, ea),
                _ => "need video+audio".to_string(),
            };
            log::info!(
                "Deferring catalog publish ({}, have {}v{}a)",
                expected,
                self.video_count,
                self.audio_count
            );
        }

        Ok(track_name)
    }

    pub fn publish_msf_catalog(&mut self) {
        let cat = self.catalog.lock();
        let mut msf = moq_mux::msf::to_msf(&cat);
        drop(cat);

        let b64 = base64::engine::general_purpose::STANDARD;
        for track in &mut msf.tracks {
            if let Some(init) = self.init_segments.get(&track.name) {
                track.init_data = Some(b64.encode(init));
            }
        }

        // Add CMSF SAP event timeline track to catalog if created
        if self.sap_track.is_some() {
            msf.tracks.push(sap_timeline_track("sap-timeline"));
        }

        match msf.to_string() {
            Ok(json) => {
                // Inject targetLatency into catalog JSON (field removed from moq_msf::Track upstream)
                let json = if let Some(latency) = self.target_latency_ms {
                    if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&json) {
                        if let Some(tracks) = val.get_mut("tracks").and_then(|t| t.as_array_mut()) {
                            for track in tracks.iter_mut() {
                                if let Some(obj) = track.as_object_mut() {
                                    obj.insert(
                                        "targetLatency".to_string(),
                                        serde_json::json!(latency),
                                    );
                                }
                            }
                        }
                        serde_json::to_string(&val).unwrap_or(json)
                    } else {
                        json
                    }
                } else {
                    json
                };

                let json_len = json.len();

                // Store catalog snapshot (without initData) in shared stats
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&json) {
                    if let Some(tracks) = val.get_mut("tracks").and_then(|t| t.as_array_mut()) {
                        for track in tracks.iter_mut() {
                            if let Some(obj) = track.as_object_mut() {
                                obj.remove("initData");
                            }
                        }
                    }
                    *self.stats.catalog_json.lock().unwrap() = Some(val);
                }

                // Log full catalog JSON periodically for debugging
                let should_log = self
                    .catalog_last_logged
                    .map(|t| t.elapsed().as_secs() >= 60)
                    .unwrap_or(true);
                if should_log {
                    log::info!("MSF catalog JSON: {}", json);
                    self.catalog_last_logged = Some(std::time::Instant::now());
                }
                match self.catalog.msf_track.append_group() {
                    Ok(mut group) => {
                        match group.write_frame(json) {
                            Ok(_) => log::debug!(
                                "MSF catalog published ({} bytes, {} tracks)",
                                json_len,
                                msf.tracks.len()
                            ),
                            Err(e) => log::info!("MSF catalog write_frame failed: {}", e),
                        }
                        let _ = group.finish();
                    }
                    Err(e) => log::info!("MSF catalog append_group failed: {}", e),
                }
            }
            Err(e) => log::info!("MSF catalog to_string failed: {}", e),
        }
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Returns true once all expected tracks have been registered.
    /// With --tracks, waits for the exact counts (e.g. 3v1a).
    /// Without --tracks, waits for any video + audio, or video-only after 5s timeout.
    pub fn has_complete_catalog(&self) -> bool {
        if let (Some(ev), Some(ea)) = (self.expected_video, self.expected_audio) {
            let video_ready = self.video_count >= ev;
            let audio_ready = if ea > 0 { self.audio_count >= ea } else { true };
            return video_ready && audio_ready;
        }

        // Default behavior: any video + any audio
        if self.video_count > 0 && self.audio_count > 0 {
            return true;
        }
        // Timeout: publish video-only catalog if no audio after 5s
        if self.video_count > 0 {
            if let Some(t) = self.first_video_at {
                return t.elapsed() > std::time::Duration::from_secs(5);
            }
        }
        false
    }

    pub fn send_fragment(&mut self, track_name: &str, data: &[u8]) -> Result<(), anyhow::Error> {
        let state = self
            .tracks
            .get_mut(track_name)
            .ok_or_else(|| anyhow!("track not found: {}", track_name))?;

        let bdt = mp4::parse_base_decode_time(data);
        let timescale = state.timescale;
        let track_type = state.track_type;

        // Track last BDT for fragment duration estimation
        if let Some(bdt_val) = bdt {
            if let Some(prev) = state.last_bdt {
                if bdt_val > prev {
                    state.last_frag_duration = Some(bdt_val - prev);
                }
            }
            state.last_bdt = Some(bdt_val);
        }

        // Determine whether to start a new group.
        // Groups are aligned with HTTP PUT boundaries (segments from the encoder).
        // http_ingest calls start_segment() at the start of each PUT, which sets
        // new_segment=true. We create a new MoQ group on that first fragment.
        let need_new_group = if state.group.is_none() {
            true
        } else {
            state.new_segment
        };
        let is_idr = need_new_group && track_type == TrackType::Video;
        if state.new_segment {
            state.new_segment = false;
        }

        if need_new_group {
            let first_group = state.group.is_none();
            if let Some(mut group) = state.group.take() {
                let _ = group.finish();
            }
            let group = if first_group {
                // MSF draft-00: first group ID = milliseconds since Unix epoch
                let epoch_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                state
                    .track
                    .create_group(Group { sequence: epoch_ms })
                    .map_err(|e| anyhow!("failed to create first group: {}", e))?
            } else {
                state
                    .track
                    .append_group()
                    .map_err(|e| anyhow!("failed to create group: {}", e))?
            };
            state.group = Some(group);
        }

        // Rebase timestamps: subtract a synchronized base so audio and video start near 0
        // with matching wall-clock alignment. The first BDT from ANY track establishes a
        // shared time origin; all other tracks compute their base from the same origin.
        let working_data = if let Some(bdt_val) = bdt {
            if state.time_base.is_none() {
                let bdt_seconds = bdt_val as f64 / timescale as f64;
                let base = if let Some(ref origin) = self.time_origin {
                    // Compute this track's base so its rebased time aligns with the origin.
                    let rebased_seconds = bdt_seconds - origin.origin_seconds;
                    let rebased_ticks = (rebased_seconds * timescale as f64).round() as i64;
                    bdt_val.saturating_sub(rebased_ticks.max(0) as u64)
                } else {
                    // First track to arrive: this BDT becomes the wall-clock origin.
                    self.time_origin = Some(SharedTimeOrigin {
                        origin_seconds: bdt_seconds,
                    });
                    bdt_val
                };
                state.time_base = Some(base);
            }
            let base = state.time_base.unwrap();

            // Log at group boundaries (after time_base is computed so values are accurate)
            if need_new_group {
                let rebased_ms = ((bdt_val.saturating_sub(base)) * 1000) / timescale as u64;
                let bdt_ms = (bdt_val * 1000) / timescale as u64;
                let base_ms = (base * 1000) / timescale as u64;
                log::debug!(
                    "{} '{}': NEW_GROUP bdt={}ms base={}ms rebased={}ms",
                    if track_type == TrackType::Video {
                        "Video"
                    } else {
                        "Audio"
                    },
                    track_name,
                    bdt_ms,
                    base_ms,
                    rebased_ms
                );
            }

            mp4::rebase_decode_time(data, base)
        } else {
            data.to_vec()
        };

        let frame_data = if let Some(dur) = state.default_sample_duration {
            Bytes::from(mp4::inject_trun_duration(&working_data, dur))
        } else {
            Bytes::from(working_data)
        };

        if let Some(ref mut group) = state.group {
            let len = frame_data.len() as u64;
            group
                .write_frame(frame_data)
                .map_err(|e| anyhow!("failed to write frame: {}", e))?;
            self.stats.bytes_published.fetch_add(len, Ordering::Relaxed);
            self.stats.frames_sent.fetch_add(1, Ordering::Relaxed);
        }

        // Emit CMSF SAP event for video fragments
        if track_type == TrackType::Video {
            if let Some(ref mut sap_track) = self.sap_track {
                let sap_type: u32 = if is_idr { 1 } else { 0 };
                let ept_ms = if let Some(bdt_val) = bdt {
                    let base = self
                        .tracks
                        .get(track_name)
                        .and_then(|s| s.time_base)
                        .unwrap_or(0);
                    let rebased = bdt_val.saturating_sub(base);
                    (rebased * 1000) / timescale as u64
                } else {
                    0
                };

                let json = sap_event_json(sap_type, ept_ms);

                // New group on IDR (aligns with media group boundaries)
                if is_idr {
                    if let Some(mut g) = self.sap_group.take() {
                        let _ = g.finish();
                    }
                    match sap_track.append_group() {
                        Ok(g) => self.sap_group = Some(g),
                        Err(e) => log::debug!("SAP timeline append_group failed: {}", e),
                    }
                }

                if let Some(ref mut group) = self.sap_group {
                    if let Err(e) = group.write_frame(Bytes::from(json)) {
                        log::debug!("SAP timeline write_frame failed: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Signal that a new HTTP PUT (segment) has started for this track.
    /// The next call to send_fragment() will create a new MoQ group.
    pub fn start_segment(&mut self, track_name: &str) {
        if let Some(state) = self.tracks.get_mut(track_name) {
            state.new_segment = true;
            self.stats.segments_sent.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Update default_sample_duration from a tfhd box in a moof fragment.
    /// Called on first fragment when the trex value was rejected as a placeholder.
    pub fn update_sample_duration_from_fragment(&mut self, track_name: &str, data: &[u8]) {
        let state = match self.tracks.get_mut(track_name) {
            Some(s) => s,
            None => return,
        };
        // Only update if we don't already have a valid value
        if state.default_sample_duration.is_some() {
            return;
        }
        if let Some(dur) = mp4::parse_tfhd_sample_duration(data) {
            log::info!(
                "Discovered default_sample_duration={} from tfhd for track '{}' (timescale={})",
                dur,
                track_name,
                state.timescale
            );
            state.default_sample_duration = Some(dur);
        }
    }

    /// Record video structure from a completed segment.
    /// `first_bdt` and `last_bdt` are the baseMediaDecodeTime of the first and last
    /// fragments, used to compute accurate media-time segment duration.
    pub fn record_segment_structure(
        &self,
        track_name: &str,
        fragment_count: u32,
        first_bdt: Option<u64>,
        last_bdt: Option<u64>,
    ) {
        // Only track video segments
        let state = match self.tracks.get(track_name) {
            Some(s) if s.track_type == TrackType::Video => s,
            _ => return,
        };

        let timescale = state.timescale;

        // Compute FPS from timescale and default_sample_duration
        let (fps, sample_dur) = if let Some(dur) = state.default_sample_duration {
            if dur > 0 {
                (timescale as f64 / dur as f64, Some(dur))
            } else {
                (0.0, None)
            }
        } else {
            (0.0, None)
        };

        // Compute segment duration from BDT span + one fragment duration
        // first_bdt..last_bdt covers (fragment_count - 1) fragments,
        // so total duration = (last_bdt - first_bdt + fragment_dur) / timescale * 1000
        let segment_duration_ms = if let (Some(first), Some(last)) = (first_bdt, last_bdt) {
            if last >= first && timescale > 0 {
                let bdt_span_ticks = last - first;
                // Add one fragment's worth of ticks to cover the last fragment's duration
                let frag_ticks = if fragment_count > 1 {
                    bdt_span_ticks / (fragment_count as u64 - 1)
                } else if let Some(dur) = sample_dur {
                    // Single fragment: estimate from sample duration × samples
                    // For now just use the BDT span (which is 0 for 1 fragment)
                    dur as u64
                } else {
                    0
                };
                let total_ticks = bdt_span_ticks + frag_ticks;
                (total_ticks * 1000) / timescale as u64
            } else {
                0
            }
        } else {
            0
        };

        let fragment_duration_ms = if fragment_count > 0 && segment_duration_ms > 0 {
            segment_duration_ms as f64 / fragment_count as f64
        } else {
            0.0
        };

        let vs = VideoStructure {
            segment_duration_ms,
            fragments_per_segment: fragment_count,
            fragment_duration_ms,
            fps,
            timescale,
            default_sample_duration: state.default_sample_duration,
        };

        *self.stats.video_structure.lock().unwrap() = Some(vs);
    }
}

// --- SAP timeline helpers (previously in moq-mux, removed upstream) ---

/// Create an MSF Track entry for a CMSF SAP-type event timeline.
fn sap_timeline_track(name: &str) -> moq_msf::Track {
    moq_msf::Track {
        name: name.to_string(),
        packaging: moq_msf::Packaging::EventTimeline,
        is_live: true,
        role: None,
        codec: None,
        width: None,
        height: None,
        framerate: None,
        samplerate: None,
        channel_config: None,
        bitrate: None,
        init_data: None,
        render_group: None,
        alt_group: None,
    }
}

/// Format a CMSF SAP event JSON payload.
fn sap_event_json(sap_type: u32, ept_ms: u64) -> String {
    format!("{{\"l\":[{},{}]}}", sap_type, ept_ms)
}
