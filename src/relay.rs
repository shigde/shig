use crate::relay::config::RelayConfig;
use crate::relay::state::RelayState;
use tokio_util::sync::CancellationToken;

pub mod config;
pub mod state;

#[cfg(test)]
mod tests;

pub struct RelayServer {
    pub state: RelayState,
    relay: moq_relay::Relay,
}

pub async fn new_relay_server(mut config: RelayConfig) -> anyhow::Result<RelayServer> {
    let _ = moq_tokio::crypto::install_default();

    if config.listen.tls.cert.is_empty()
        && config.listen.tls.key.is_empty()
        && config.listen.tls.generate.is_empty()
        && config.listen.tls.identity.is_none()
    {
        config.listen.tls.generate = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    }

    let relay = Box::pin(moq_relay::Relay::load(config))
        .await?
        .with_signals(false);
    let state = RelayState {
        auth: relay.auth().clone(),
        cluster: relay.cluster().clone(),
        web: relay.web().routes(),
    };

    Ok(RelayServer { state, relay })
}

pub async fn start_moq_udp_only(
    relay: RelayServer,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let trigger = relay.relay.shutdown_trigger().clone();
    let mut running = Box::pin(relay.relay.run());
    tokio::select! {
        result = &mut running => result,
        _ = shutdown.cancelled() => {
            trigger.start();
            running.await
        }
    }
}
