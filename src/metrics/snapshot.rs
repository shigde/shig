/// Combines component snapshots for the dashboard API.
///
/// Flattening the current SFU snapshot preserves the existing response shape.
/// New component snapshots can be added here without moving their collection logic.
#[derive(serde::Serialize)]
pub(crate) struct MonitoringSnapshot {
    #[serde(flatten)]
    sfu: crate::sfu::metrics::MonitoringSnapshot,
    relay: super::relay::RelaySnapshot,
    recorder: super::recorder::RecorderSnapshot,
    system: super::system::SystemSnapshot,
}

pub(crate) fn snapshot(relay: &moq_net::stats::Registry) -> MonitoringSnapshot {
    MonitoringSnapshot {
        sfu: crate::sfu::metrics::snapshot(),
        relay: super::relay::snapshot(relay),
        recorder: super::recorder::snapshot(),
        system: super::system::snapshot(),
    }
}
