use crate::relay::config::RelayConfig;
use crate::relay::state::RelayState;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

pub mod config;
mod mp4;
mod publisher;
pub mod state;
mod stats;
mod track_state;

pub struct RelayServer {
    pub state: RelayState,
    server: moq_native::Server,
    cluster: moq_relay::Cluster,
    auth: moq_relay::Auth,
}

pub async fn new_relay_server(mut config: RelayConfig) -> anyhow::Result<RelayServer> {
    use moq_relay::*;

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install default crypto provider");

    config.client.max_streams.get_or_insert(DEFAULT_MAX_STREAMS);
    config.server.max_streams.get_or_insert(DEFAULT_MAX_STREAMS);

    if config.server.tls.cert.is_empty()
        && config.server.tls.key.is_empty()
        && config.server.tls.generate.is_empty()
    {
        config.server.tls.generate = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    }

    if config.auth.key.is_none() && config.auth.public.is_none() {
        let pc = PublicConfig::from_str("")?;
        config.auth.public = Some(pc);
    }

    let server = config.server.init()?;
    let client = config.client.init()?;

    let auth = config.auth.init().await?;
    let cluster = Cluster::new(config.cluster, client);

    let state = RelayState {
        auth: auth.clone(),
        cluster: cluster.clone(),
        tls_info: server.tls_info(),
        conn_id: Arc::new(AtomicU64::new(0)),
    };

    Ok(RelayServer {
        state,
        server,
        cluster,
        auth,
    })
}

pub async fn start_moq_udp_only(relay: RelayServer) -> anyhow::Result<()> {
    use anyhow::Context;

    let RelayServer {
        server,
        cluster,
        auth,
        ..
    } = relay;

    tokio::select! {
        Err(err) = cluster.clone().run() => Err(err).context("cluster failed"),
        Err(err) = serve(server, cluster, auth) => Err(err).context("server failed"),
        else => Ok(()),
    }
}

async fn serve(
    mut server: moq_native::Server,
    cluster: moq_relay::Cluster,
    auth: moq_relay::Auth,
) -> anyhow::Result<()> {
    let mut conn_id = 0;

    log::info!(
        "relay server listening on {}",
        server.local_addr()?.to_string()
    );

    while let Some(request) = server.accept().await {
        let conn = moq_relay::Connection {
            id: conn_id,
            request,
            cluster: cluster.clone(),
            auth: auth.clone(),
        };

        conn_id += 1;

        tokio::spawn(async move {
            if let Err(err) = conn.run().await {
                tracing::warn!(%err, "connection closed");
            }
        });
    }

    anyhow::bail!("stopped accepting connections")
}
