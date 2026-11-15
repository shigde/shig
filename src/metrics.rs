mod snapshot;

pub(crate) use snapshot::snapshot;

use prometheus::{Encoder, TextEncoder};

/// Exports metrics registered by the server's metric-owning components.
pub(crate) fn render() -> Result<String, prometheus::Error> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}
