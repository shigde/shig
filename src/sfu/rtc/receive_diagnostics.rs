use std::collections::HashMap;
use std::time::{Duration, Instant};

const WINDOW: i64 = 4096;

/// Observes application delivery after the interceptors AND TrackRemote's event queue.
/// Gaps are provisional; missing packets can arrive in a later reporting window.
#[derive(Default)]
pub(super) struct ReceiveDiagnostics {
    highest: Option<i64>,
    missing: std::collections::BTreeSet<i64>,
    previous: Option<Instant>,
    packets: u64,
    gaps: u64,
    late: u64,
    old_or_duplicate: u64,
    expired: u64,
    discontinuities: u64,
    max_gap: Duration,
    close_pairs: u64,
}

impl ReceiveDiagnostics {
    pub fn observe(&mut self, sequence: u16, now: Instant) {
        self.packets += 1;
        if let Some(previous) = self.previous {
            let gap = now.saturating_duration_since(previous);
            self.max_gap = self.max_gap.max(gap);
            if gap <= Duration::from_millis(1) { self.close_pairs += 1; }
        }
        self.previous = Some(now);
        let Some(highest) = self.highest else {
            self.highest = Some(i64::from(sequence));
            return;
        };
        let delta = sequence.wrapping_sub(highest as u16) as i16 as i64;
        let extended = highest + delta;
        if delta > WINDOW {
            // A restart or large jump is ambiguous; do not call thousands of packets lost.
            self.discontinuities += 1;
            self.expired += self.missing.len() as u64;
            self.missing.clear();
            self.highest = Some(extended);
        } else if delta > 0 {
            self.gaps += (delta - 1) as u64;
            self.missing.extend(highest + 1..extended);
            self.highest = Some(extended);
            while self.missing.first().is_some_and(|old| *old < extended - WINDOW) {
                self.missing.pop_first();
                self.expired += 1;
            }
        } else if self.missing.remove(&extended) {
            self.late += 1;
        } else {
            self.old_or_duplicate += 1;
        }
    }

    pub fn report(&mut self, media: &impl std::fmt::Display, ssrc: u32) {
        log::debug!(
            "RTP receive delivery: media_id={}, ssrc={}, packets={}, delivery_gap_max_ms={:.3}, delivery_pairs_le_1ms={}, sequence_gaps={}, late_gap_fills={}, missing_pending={}, old_or_duplicate={}, missing_aged_out={}, sequence_discontinuities={}",
            media, ssrc, self.packets, self.max_gap.as_secs_f64() * 1000.0,
            self.close_pairs, self.gaps, self.late, self.missing.len(),
            self.old_or_duplicate, self.expired, self.discontinuities,
        );
        self.packets = 0;
        self.gaps = 0;
        self.late = 0;
        self.old_or_duplicate = 0;
        self.expired = 0;
        self.discontinuities = 0;
        self.max_gap = Duration::ZERO;
        self.close_pairs = 0;
    }
}

pub(super) type TrackDiagnostics = HashMap<u32, ReceiveDiagnostics>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraparound_late_repair_and_duplicate_are_distinguished() {
        let now = Instant::now();
        let mut stats = ReceiveDiagnostics::default();
        for sequence in [65534, 1, 65535, 0, 0, 2] { stats.observe(sequence, now); }
        assert_eq!(stats.gaps, 2);
        assert_eq!(stats.late, 2);
        assert_eq!(stats.old_or_duplicate, 1);
        assert!(stats.missing.is_empty());
    }

    #[test]
    fn pending_gaps_survive_reports_and_tracking_is_bounded() {
        let now = Instant::now();
        let mut stats = ReceiveDiagnostics::default();
        stats.observe(0, now);
        stats.observe(2, now + Duration::from_millis(120));
        assert_eq!(stats.max_gap, Duration::from_millis(120));
        stats.report(&"test", 1);
        stats.observe(1, now + Duration::from_millis(121));
        assert_eq!(stats.late, 1);
        for seq in (4..20000).step_by(2) { stats.observe(seq, now); }
        assert!(stats.missing.len() <= WINDOW as usize);
        assert!(stats.expired > 0);
        stats.observe(30000, now);
        assert_eq!(stats.discontinuities, 1);
        assert!(stats.missing.is_empty());
    }
}
