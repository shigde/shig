mod snapshot;
pub(crate) mod recorder;
mod relay;
mod system;

pub(crate) use snapshot::snapshot;

/// Starts the low-frequency operating-system metrics sampler.
pub(crate) fn init() {
    recorder::init();
    system::init();
}

use prometheus::{Encoder, TextEncoder};

/// Exports metrics registered by the server's metric-owning components.
pub(crate) async fn render(
    relay: Option<&crate::relay::state::RelayState>,
) -> Result<String, String> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .map_err(|error| error.to_string())?;
    let mut body = String::from_utf8_lossy(&buffer).into_owned();
    if let Some(relay) = relay {
        body.push_str(
            &relay
                .prometheus_metrics()
                .await
                .map_err(|error| error.to_string())?,
        );
    }
    Ok(body)
}
