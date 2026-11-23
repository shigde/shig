use std::{sync::Mutex, time::{Duration, Instant}};

/// Shared by all keyframe request sources of one Media. No timer or packet queue.
#[derive(Default)]
pub(super) struct PliGate(Mutex<Option<Instant>>);

impl PliGate {
    pub(super) fn admit(&self, now: Instant) -> bool {
        let mut last = self.0.lock().unwrap();
        if last.is_some_and(|last| now.duration_since(last) < Duration::from_millis(200)) {
            return false;
        }
        *last = Some(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn subscribers_share_a_window_but_other_source_tracks_do_not() {
        let source = Arc::new(PliGate::default());
        let other_subscriber = source.clone();
        let other_source = PliGate::default();
        let now = Instant::now();
        assert!(source.admit(now));
        assert!(!other_subscriber.admit(now + Duration::from_millis(199)));
        assert!(other_source.admit(now));
        assert!(other_subscriber.admit(now + Duration::from_millis(200)));
        assert!(!source.admit(now + Duration::from_millis(201)));
        assert!(source.admit(now + Duration::from_secs(1)));
    }

    #[test]
    fn concurrent_requests_only_admit_one_subscriber() {
        let gate = Arc::new(PliGate::default());
        let now = Instant::now();
        let handles: Vec<_> = (0..16).map(|_| {
            let gate = gate.clone();
            std::thread::spawn(move || gate.admit(now))
        }).collect();
        assert_eq!(handles.into_iter().filter_map(|h| h.join().ok()).filter(|&v| v).count(), 1);
    }
}
