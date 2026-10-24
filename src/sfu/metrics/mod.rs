mod active_lobbies;
mod active_lobby_peers;
mod snapshot;

use prometheus::{Encoder, TextEncoder};

pub(crate) use active_lobbies::set_active_lobbies;
pub(crate) use active_lobby_peers::{remove_active_lobby_peers, set_active_lobby_peers};
pub(crate) use snapshot::snapshot;

pub(crate) fn init() {
    active_lobbies::active_lobbies();
    active_lobby_peers::active_lobby_peers();
}

pub(crate) fn render() -> Result<String, prometheus::Error> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}
