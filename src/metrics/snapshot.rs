/// Combines component snapshots for the dashboard API.
///
/// Flattening the current SFU snapshot preserves the existing response shape.
/// New component snapshots can be added here without moving their collection logic.
#[derive(serde::Serialize)]
pub(crate) struct MonitoringSnapshot {
    #[serde(flatten)]
    sfu: crate::sfu::metrics::MonitoringSnapshot,
}

pub(crate) fn snapshot() -> MonitoringSnapshot {
    MonitoringSnapshot {
        sfu: crate::sfu::metrics::snapshot(),
    }
}
