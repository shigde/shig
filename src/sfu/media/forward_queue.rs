use rtp::packet::Packet;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub(super) const CAPACITY: usize = 512;

/// One bounded oldest-first eviction ring per subscriber track.
/// The sender task never owns this object, so removing the route stops it.
pub(super) struct ForwardQueue {
    sender: broadcast::Sender<Arc<Packet>>,
    receiver: Mutex<Option<broadcast::Receiver<Arc<Packet>>>>,
    receiver_taken: AtomicBool,
    pub cancel: CancellationToken,
}

impl ForwardQueue {
    pub fn new() -> Self {
        let (sender, receiver) = broadcast::channel(CAPACITY);
        Self { sender, receiver: Mutex::new(Some(receiver)), receiver_taken: AtomicBool::new(false), cancel: CancellationToken::new() }
    }

    pub fn take_receiver(&self) -> Option<broadcast::Receiver<Arc<Packet>>> {
        if self.receiver_taken.load(Ordering::Acquire)
            || self.receiver_taken.swap(true, Ordering::AcqRel) { return None; }
        self.receiver.lock().unwrap().take()
    }

    pub fn enqueue(&self, packet: Arc<Packet>) {
        if !self.cancel.is_cancelled() { let _ = self.sender.send(packet); }
    }
}

impl Drop for ForwardQueue {
    fn drop(&mut self) { self.cancel.cancel(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(sequence_number: u16) -> Arc<Packet> {
        Arc::new(Packet { header: rtp::header::Header { sequence_number, ..Default::default() }, ..Default::default() })
    }

    #[tokio::test]
    async fn a_stalled_subscriber_loses_oldest_packets_without_blocking_another() {
        let slow = ForwardQueue::new();
        let fast = ForwardQueue::new();
        let mut slow_rx = slow.take_receiver().unwrap();
        let mut fast_rx = fast.take_receiver().unwrap();
        for sequence in 0..600 {
            slow.enqueue(packet(sequence));
            fast.enqueue(packet(sequence));
            assert_eq!(fast_rx.recv().await.unwrap().header.sequence_number, sequence);
        }
        assert_eq!(slow_rx.len(), 600);
        assert!(matches!(slow_rx.recv().await, Err(broadcast::error::RecvError::Lagged(88))));
        for sequence in 88..600 {
            assert_eq!(slow_rx.recv().await.unwrap().header.sequence_number, sequence);
        }
        assert!(slow_rx.is_empty());
    }

    #[test]
    fn last_route_drop_cancels_its_sender_and_receiver_is_taken_only_once() {
        let queue = std::sync::Arc::new(ForwardQueue::new());
        let clone = queue.clone();
        let cancel = queue.cancel.clone();
        assert!(queue.take_receiver().is_some());
        assert!(clone.take_receiver().is_none());
        drop(queue);
        assert!(!cancel.is_cancelled());
        drop(clone);
        assert!(cancel.is_cancelled());
    }
}
