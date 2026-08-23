use crate::sfu::endpoint::RtcEndpointId;
use crate::sfu::peer::PeerId;
use rtc::rtp_transceiver::{RTCRtpSenderId, SSRC};
use std::collections::{HashMap, HashSet};

/// Identity of one publishing track being forwarded: the publishing endpoint plus the
/// **mid** of its m-line. The mid is stable across the publisher's renegotiations
/// (a stopped/muted track keeps its m-line and only flips direction), so it — not the
/// SSRC — is the correct dedup key for the forwarding graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ForwardKey {
    pub(crate) publisher_peer: PeerId,
    pub(crate) publish_endpoint: RtcEndpointId,
    pub(crate) mid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForwardTarget {
    pub(crate) subscriber_peer: PeerId,
    pub(crate) subscribe_endpoint: RtcEndpointId,
    pub(crate) sender_id: RTCRtpSenderId,
    pub(crate) payload_types: HashMap<u8, u8>,
    pub(crate) extension_rewrites: Vec<HeaderExtensionRewrite>,
    pub(crate) subscriber_mid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HeaderExtensionRewrite {
    pub(crate) publisher_id: u8,
    pub(crate) subscriber_id: u8,
    pub(crate) rewrite_mid: bool,
}

/// The `(publisher mid) x (subscriber)` forwarding matrix, plus the wire-level routing
/// index that maps a publisher's RTP SSRC to its forward key.
///
/// For each publish track the matrix records which subscribers already have a forwarding
/// sender and the `RTCRtpSenderId` of that sender on the subscriber's peer connection
/// (needed both to tear it down and to route packets to it). This is the dedup state that
/// makes track extraction idempotent: a publisher re-offering the same tracks must not
/// add duplicate senders.
///
/// The SSRC index is what turns an inbound `RtpPacket`/`RtcpPacket` (identified on the
/// wire only by SSRC) into the set of subscriber senders it fans out to. It is seeded
/// from the SDP at reconcile time when the offer names the SSRC (`a=ssrc`), and completed
/// from the publisher's `OnTrack(OnOpen)` otherwise (bare m-line, RID-based simulcast).
#[derive(Debug, Default)]
pub(crate) struct ForwardTable {
    entries: HashMap<ForwardKey, HashMap<PeerId, ForwardTarget>>,
    ssrc_index: HashMap<SSRC, ForwardKey>,
}

impl ForwardTable {
    /// Record a newly created forwarding sender.
    pub(crate) fn insert(&mut self, key: ForwardKey, target: ForwardTarget) {
        self.entries
            .entry(key)
            .or_default()
            .insert(target.subscriber_peer.clone(), target);
    }

    pub(crate) fn subscriber_sender(
        &self,
        key: &ForwardKey,
        subscriber: &PeerId,
    ) -> Option<RTCRtpSenderId> {
        Some(self.entries.get(key)?.get(subscriber)?.sender_id)
    }

    pub(crate) fn update_subscriber_target(&mut self, key: &ForwardKey, target: ForwardTarget) {
        if let Some(subscribers) = self.entries.get_mut(key) {
            subscribers.insert(target.subscriber_peer.clone(), target);
        }
    }

    /// Bind a publisher's wire SSRC to its forward key so inbound packets carrying that
    /// SSRC can be routed. Idempotent; re-binding an SSRC follows the publisher's latest
    /// negotiation. Called from reconcile (SSRC known from `a=ssrc`) or from the
    /// publisher's `OnTrack(OnOpen)` (packet-time binding for bare m-lines / simulcast
    /// RID layers — each simulcast layer's SSRC binds to the same key).
    pub(crate) fn bind_ssrc(&mut self, ssrc: SSRC, key: ForwardKey) {
        self.ssrc_index.insert(ssrc, key);
    }

    /// Resolve a packet's SSRC to its forward key and the subscriber senders it fans out
    /// to. `None` until the SSRC is bound and at least one subscriber sender exists.
    pub(crate) fn route_by_ssrc(
        &self,
        ssrc: SSRC,
    ) -> Option<(&ForwardKey, &HashMap<PeerId, ForwardTarget>)> {
        let key = self.ssrc_index.get(&ssrc)?;
        let subscribers = self.entries.get(key)?;
        Some((key, subscribers))
    }

    /// Drop forwardings that are no longer wanted and collect their senders so the caller
    /// can `remove_track` them from the subscriber peer connections:
    ///   - the `(publisher, mid)` is no longer published (not in `desired`),
    ///   - the publisher has left the lobby or is no longer a publish endpoint, or
    ///   - the subscriber has left the lobby or is no longer a subscribe endpoint.
    ///
    /// SSRC bindings whose key vanished are pruned with it. Everything still wanted is
    /// kept, so re-running this with an unchanged lobby is a no-op (the intersection case).
    pub(crate) fn retain(
        &mut self,
        desired: &HashSet<ForwardKey>,
        live_publishers: &HashSet<RtcEndpointId>,
        live_subscribers: &HashSet<RtcEndpointId>,
        removed: &mut Vec<(RtcEndpointId, RTCRtpSenderId)>,
    ) {
        self.entries.retain(|key, subs| {
            let key_alive = desired.contains(key) && live_publishers.contains(&key.publish_endpoint);
            subs.retain(|_, target| {
                let keep = key_alive
                    && live_subscribers.contains(&target.subscribe_endpoint)
                    && key.publisher_peer != target.subscriber_peer;
                if !keep {
                    removed.push((target.subscribe_endpoint, target.sender_id));
                }
                keep
            });
            !subs.is_empty()
        });

        let entries = &self.entries;
        self.ssrc_index.retain(|_, key| entries.contains_key(key));
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.ssrc_index.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(publisher: RtcEndpointId, mid: &str) -> ForwardKey {
        ForwardKey {
            publisher_peer: PeerId::new(format!("publisher-{publisher}")),
            publish_endpoint: publisher,
            mid: mid.to_owned(),
        }
    }

    fn target(peer: &str, endpoint: RtcEndpointId, sender_id: usize) -> ForwardTarget {
        ForwardTarget {
            subscriber_peer: PeerId::new(peer),
            subscribe_endpoint: endpoint,
            sender_id: RTCRtpSenderId::from(sender_id),
            payload_types: HashMap::new(),
            extension_rewrites: Vec::new(),
            subscriber_mid: None,
        }
    }

    #[test]
    fn routes_bound_ssrc_to_subscriber_senders() {
        let mut table = ForwardTable::default();
        let k = key(1, "0");
        table.insert(k.clone(), target("subscriber", 2, 7));
        table.bind_ssrc(1111, k.clone());

        let (routed_key, subscribers) = table.route_by_ssrc(1111).expect("ssrc should route");
        assert_eq!(routed_key, &k);
        assert_eq!(
            subscribers.get(&PeerId::new("subscriber")).map(|target| target.sender_id),
            Some(RTCRtpSenderId::from(7))
        );

        // Unbound SSRC routes nowhere.
        assert!(table.route_by_ssrc(2222).is_none());
    }

    #[test]
    fn simulcast_layers_bind_to_the_same_key() {
        let mut table = ForwardTable::default();
        let k = key(1, "0");
        table.insert(k.clone(), target("subscriber", 2, 7));
        // Two layers, learned at packet time (OnTrack per RID).
        table.bind_ssrc(1111, k.clone());
        table.bind_ssrc(1112, k.clone());

        assert!(table.route_by_ssrc(1111).is_some());
        assert!(table.route_by_ssrc(1112).is_some());
    }

    #[test]
    fn retain_prunes_ssrc_bindings_with_their_key() {
        let mut table = ForwardTable::default();
        let k = key(1, "0");
        table.insert(k.clone(), target("subscriber", 2, 7));
        table.bind_ssrc(1111, k.clone());

        // Publisher 1 gone: entry and its SSRC binding must both go.
        let mut removed = Vec::new();
        let desired = HashSet::from([k]);
        let live_publishers = HashSet::new();
        let live_subscribers = HashSet::from([2]);
        table.retain(
            &desired,
            &live_publishers,
            &live_subscribers,
            &mut removed,
        );

        assert_eq!(removed, vec![(2, RTCRtpSenderId::from(7))]);
        assert!(table.route_by_ssrc(1111).is_none());
        assert!(table.entries.is_empty());
    }

    #[test]
    fn retain_prunes_self_forwardings() {
        let mut table = ForwardTable::default();
        let k = ForwardKey {
            publisher_peer: PeerId::new("same-peer"),
            publish_endpoint: 1,
            mid: "0".to_owned(),
        };
        table.insert(k.clone(), target("same-peer", 2, 7));
        table.bind_ssrc(1111, k.clone());

        let mut removed = Vec::new();
        let desired = HashSet::from([k]);
        let live_publishers = HashSet::from([1]);
        let live_subscribers = HashSet::from([2]);
        table.retain(
            &desired,
            &live_publishers,
            &live_subscribers,
            &mut removed,
        );

        assert_eq!(removed, vec![(2, RTCRtpSenderId::from(7))]);
        assert!(table.route_by_ssrc(1111).is_none());
        assert!(table.entries.is_empty());
    }
}
