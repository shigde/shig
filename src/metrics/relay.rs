use moq_net::stats::{Registry, Snapshot};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct RelaySnapshot {
    traffic: Vec<TrafficSnapshot>,
    sessions: Vec<SessionSnapshot>,
}

#[derive(Serialize)]
struct TrafficSnapshot {
    tier: String,
    role: String,
    announces_started: u64,
    announces_ended: u64,
    announced_bytes: u64,
    viewers_opened: u64,
    viewers_closed: u64,
    active_viewers: u64,
    subscriptions_opened: u64,
    subscriptions_closed: u64,
    active_subscriptions: u64,
    fetches: u64,
    bytes: u64,
    frames: u64,
    groups: u64,
    datagrams: u64,
    stale_bytes: u64,
    stale_frames: u64,
    stale_groups: u64,
    stale_datagrams: u64,
}

#[derive(Serialize)]
struct SessionSnapshot {
    tier: String,
    opened: u64,
    closed: u64,
    active: u64,
}

pub(crate) fn snapshot(registry: &Registry) -> RelaySnapshot {
    let snapshot = registry.snapshot();
    RelaySnapshot {
        traffic: traffic_snapshots(&snapshot),
        sessions: session_snapshots(&snapshot),
    }
}

fn traffic_snapshots(snapshot: &Snapshot) -> Vec<TrafficSnapshot> {
    snapshot
        .traffic()
        .into_iter()
        .map(|(tier, role, totals)| TrafficSnapshot {
            tier: tier.as_str().to_owned(),
            role: role.as_str().to_owned(),
            announces_started: totals.announces_started,
            announces_ended: totals.announces_ended,
            announced_bytes: totals.announced_bytes,
            viewers_opened: totals.broadcasts_started,
            viewers_closed: totals.broadcasts_ended,
            active_viewers: totals
                .broadcasts_started
                .saturating_sub(totals.broadcasts_ended),
            subscriptions_opened: totals.subscriptions_started,
            subscriptions_closed: totals.subscriptions_ended,
            active_subscriptions: totals.active_subscriptions(),
            fetches: totals.fetches,
            bytes: totals.bytes,
            frames: totals.frames,
            groups: totals.groups,
            datagrams: totals.datagrams,
            stale_bytes: totals.stale.bytes,
            stale_frames: totals.stale.frames,
            stale_groups: totals.stale.groups,
            stale_datagrams: totals.stale.datagrams,
        })
        .collect()
}

fn session_snapshots(snapshot: &Snapshot) -> Vec<SessionSnapshot> {
    snapshot
        .sessions()
        .into_iter()
        .map(|(tier, presence)| SessionSnapshot {
            tier: tier.as_str().to_owned(),
            opened: presence.sessions_started,
            closed: presence.sessions_ended,
            active: presence
                .sessions_started
                .saturating_sub(presence.sessions_ended),
        })
        .collect()
}

