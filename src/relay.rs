use crate::relay::config::RelayConfig;

pub mod config;
mod mp4;
mod publisher;
mod stats;
mod track_state;

pub async fn start_moq_udp_only(mut config: RelayConfig) -> anyhow::Result<()> {
    use anyhow::Context;
    use moq_relay::*;

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install default crypto provider");

    config.client.max_streams.get_or_insert(DEFAULT_MAX_STREAMS);
    config.server.max_streams.get_or_insert(DEFAULT_MAX_STREAMS);

    config.server.tls.generate = vec!["localhost".to_string(), "127.0.0.1".to_string()];

    if config.auth.key.is_none() && config.auth.public.is_none() {
        config.auth.public = Some("".to_string());
    }

    let server = config.server.init()?;
    let client = config.client.init()?;

    #[cfg(feature = "iroh")]
    let (server, client) = {
        let iroh = config.iroh.bind().await?;
        (server.with_iroh(iroh.clone()), client.with_iroh(iroh))
    };

    let auth = config.auth.init().await?;
    let cluster = Cluster::new(config.cluster, client);

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

    log::info!("relay server listening on {}", server.local_addr()?.to_string());

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
