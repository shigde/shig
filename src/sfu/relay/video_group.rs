use std::time::{Duration, Instant};

// A browser need not produce periodic keyframes on its own. Ask before a GOP
// can grow into moq-net's bounded group cache; only real keyframes split video.
#[derive(Default)]
pub(super) struct VideoGroup {
    started: Option<Instant>,
    last_request: Option<Instant>,
    first_request: Option<Instant>,
    bytes: u64,
    frames: usize,
    response_time: Duration,
}

impl VideoGroup {
    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn observe(&mut self, keyframe: bool, bytes: usize, now: Instant) -> bool {
        if keyframe {
            if let Some(requested) = self.first_request.take() {
                // Keep headroom for encoder and transport variability, not just
                // the response time of the last successful request.
                self.response_time = now
                    .duration_since(requested)
                    .saturating_mul(2)
                    .clamp(Duration::from_millis(500), Duration::from_secs(5));
            }
            self.started = Some(now);
            self.last_request = None;
            self.bytes = 0;
            self.frames = 0;
        }
        self.bytes = self.bytes.saturating_add(bytes as u64);
        self.frames = self.frames.saturating_add(1);
        if keyframe {
            return false;
        }
        self.poll(now)
    }

    pub fn requested(&mut self, now: Instant) {
        self.first_request.get_or_insert(now);
        self.last_request = Some(now);
    }

    pub fn poll(&mut self, now: Instant) -> bool {
        let reserve = self.response_time.max(Duration::from_millis(500));
        let due = match self.started {
            None => true,
            Some(started) => {
                let age = now.duration_since(started);
                // Average growth predicts how much more data can arrive while
                // the publisher reacts. Never wait beyond 75% of either limit.
                let growth = reserve.as_secs_f64() / age.as_secs_f64().max(0.1);
                let projected_bytes = self.bytes as f64 * (1.0 + growth);
                let projected_frames = self.frames as f64 * (1.0 + growth);
                age + reserve.min(Duration::from_secs(1)) >= Duration::from_secs(2)
                    || projected_bytes >= moq_net::group::MAX_CACHE_BYTES as f64 * 0.75
                    || projected_frames >= moq_net::group::MAX_GROUP_FRAMES as f64 * 0.75
            }
        };
        let retry = self
            .last_request
            .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1));
        if due && retry {
            self.requested(now);
            return true;
        }
        false
    }
}
