use crate::relay::config::RelayConfig;
use crate::relay::state::RelayState;
use futures::future::BoxFuture;
use tokio_util::sync::CancellationToken;

pub mod config;
pub mod state;

#[cfg(test)]
mod tests;

pub struct RelayServer {
    pub state: RelayState,
    relay: moq_relay::Relay,
    authorization: Option<BoxFuture<'static, anyhow::Result<()>>>,
}

impl RelayServer {
    pub fn with_authorizer<F, Fut>(mut self, authorize: F) -> anyhow::Result<Self>
    where
        F: FnOnce(moq_relay::auth::Admissions) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let admissions = self.relay.admissions().ok_or_else(|| {
            anyhow::anyhow!("Embedded authorization requires empty relay.auth settings")
        })?;
        self.authorization = Some(Box::pin(authorize(admissions)));
        Ok(self)
    }
}

pub async fn new_relay_server(mut config: RelayConfig) -> anyhow::Result<RelayServer> {
    let _ = moq_tokio::crypto::install_default();

    // The relay's traffic registry is a no-op unless stats are enabled. Shig
    // reads that registry for the admin monitoring snapshot and Prometheus
    // endpoint, so enable the upstream stats producer and counter collection.
    config.stats.enabled = true;

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
    let relay_stats = relay.stats().registry().clone();
    let relay_metrics = relay.internal().routes();
    let state = RelayState {
        h264_diagnostics_directory: None,
        media: Default::default(),
        relay_stats,
        relay_metrics,
        auth: relay.auth().clone(),
        cluster: relay.cluster().clone(),
        web: relay.web().routes(),
    };

    Ok(RelayServer {
        state,
        relay,
        authorization: None,
    })
}

pub async fn start_moq_udp_only(
    relay: RelayServer,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let trigger = relay.relay.shutdown_trigger().clone();
    let mut running = Box::pin(relay.relay.run());
    let authorization = async move {
        match relay.authorization {
            Some(authorize) => authorize.await,
            None => std::future::pending().await,
        }
    };
    tokio::select! {
        result = &mut running => result,
        result = authorization => {
            trigger.start();
            let _ = running.await;
            result.and_then(|()| anyhow::bail!("Relay authorization stopped unexpectedly"))
        }
        _ = shutdown.cancelled() => {
            trigger.start();
            running.await
        }
    }
}
