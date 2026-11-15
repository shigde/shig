mod active_lobbies;
mod active_lobby_peers;
mod snapshot;

pub(crate) use active_lobbies::set_active_lobbies;
pub(crate) use active_lobby_peers::{remove_active_lobby_peers, set_active_lobby_peers};
pub(crate) use snapshot::{snapshot, MonitoringSnapshot};

pub(crate) fn init() {
    active_lobbies::active_lobbies();
    active_lobby_peers::active_lobby_peers();
}

