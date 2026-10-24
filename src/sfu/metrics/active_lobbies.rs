use prometheus::{default_registry, IntGauge};
use std::sync::OnceLock;

static ACTIVE_LOBBIES: OnceLock<IntGauge> = OnceLock::new();

pub(crate) fn set_active_lobbies(count: usize) {
    active_lobbies().set(count as i64);
}

pub(super) fn active_lobbies() -> &'static IntGauge {
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
