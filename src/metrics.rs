use prometheus::{default_registry, Encoder, IntGauge, IntGaugeVec, Opts, TextEncoder};
use std::sync::OnceLock;

static ACTIVE_LOBBIES: OnceLock<IntGauge> = OnceLock::new();
static ACTIVE_LOBBY_PEERS: OnceLock<IntGaugeVec> = OnceLock::new();

pub(crate) fn init() {
    active_lobbies();
    active_lobby_peers();
}

pub(crate) fn render() -> Result<String, prometheus::Error> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

pub(crate) fn set_active_lobbies(count: usize) {
    active_lobbies().set(count as i64);
}

pub(crate) fn set_active_lobby_peers(lobby_id: &str, count: usize) {
    active_lobby_peers()
        .with_label_values(&[lobby_id])
        .set(count as i64);
}

pub(crate) fn remove_active_lobby_peers(lobby_id: &str) {
    let _ = active_lobby_peers().remove_label_values(&[lobby_id]);
}

fn active_lobbies() -> &'static IntGauge {
    ACTIVE_LOBBIES.get_or_init(|| {
        let gauge = IntGauge::new(
            "shig_sfu_active_lobbies",
            "Current number of active SFU lobbies.",
        )
        .expect("valid active lobbies gauge");
        default_registry()
            .register(Box::new(gauge.clone()))
            .expect("register active lobbies gauge");
        gauge
    })
}

fn active_lobby_peers() -> &'static IntGaugeVec {
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
