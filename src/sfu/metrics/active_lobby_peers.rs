use prometheus::{default_registry, IntGaugeVec, Opts};
use std::sync::OnceLock;

static ACTIVE_LOBBY_PEERS: OnceLock<IntGaugeVec> = OnceLock::new();

pub(crate) fn set_active_lobby_peers(lobby_id: &str, count: usize) {
    active_lobby_peers()
        .with_label_values(&[lobby_id])
        .set(count as i64);
}

pub(crate) fn remove_active_lobby_peers(lobby_id: &str) {
    let _ = active_lobby_peers().remove_label_values(&[lobby_id]);
}

pub(super) fn active_lobby_peers() -> &'static IntGaugeVec {
    ACTIVE_LOBBY_PEERS.get_or_init(|| {
        let gauge = IntGaugeVec::new(
            Opts::new(
                "shig_sfu_active_lobby_peers",
                "Current number of active peers per SFU lobby.",
            ),
            &["lobby_id"],
        )
        .expect("valid active lobby peers gauge");
        default_registry()
            .register(Box::new(gauge.clone()))
            .expect("register active lobby peers gauge");
        gauge
    })
}
