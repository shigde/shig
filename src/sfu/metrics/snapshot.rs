use super::active_lobbies::active_lobbies;
use super::active_lobby_peers::active_lobby_peers;

#[derive(serde::Serialize)]
pub(crate) struct LobbySnapshot {
    lobby_id: String,
    peers: i64,
}

#[derive(serde::Serialize)]
pub(crate) struct MonitoringSnapshot {
    active_lobbies: i64,
    active_peers: i64,
    lobbies: Vec<LobbySnapshot>,
    version: &'static str,
}

pub(crate) fn snapshot() -> MonitoringSnapshot {
    use prometheus::core::Collector;
    let mut lobbies = Vec::new();
    for family in active_lobby_peers().collect() {
        for metric in family.get_metric() {
            if let Some(label) = metric
                .get_label()
                .iter()
                .find(|label| label.get_name() == "lobby_id")
            {
                lobbies.push(LobbySnapshot {
                    lobby_id: label.get_value().to_owned(),
                    peers: metric.get_gauge().get_value() as i64,
                });
            }
        }
    }
    lobbies.sort_by(|a, b| a.lobby_id.cmp(&b.lobby_id));
    MonitoringSnapshot {
        active_lobbies: active_lobbies().get(),
        active_peers: lobbies.iter().map(|lobby| lobby.peers).sum(),
        lobbies,
        version: env!("CARGO_PKG_VERSION"),
    }
}
