use crate::relay::config::RelayConfig;
use crate::relay::state::RelayState;
use std::str::FromStr;
use tokio_util::sync::CancellationToken;

pub mod config;
pub mod state;

pub struct RelayServer {
    pub state: RelayState,
    server: moq_native::Server,
    web: moq_relay::Web,
}

pub async fn new_relay_server(mut config: RelayConfig) -> anyhow::Result<RelayServer> {
    use moq_relay::*;

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install default crypto provider");

    config.client.max_streams.get_or_insert(DEFAULT_MAX_STREAMS);
    config.server.max_streams.get_or_insert(DEFAULT_MAX_STREAMS);

    // let mtls_enabled = !config.server.tls.root.is_empty();

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
    let client = config.client.clone().init()?;

    let (server, client) = {
        let iroh = config.iroh.bind().await?;
        (server.with_iroh(iroh.clone()), client.with_iroh(iroh))
    };

    let auth = config.auth.init().await?;

    let cluster = Cluster::new(config.cluster)
        .with_client(client)
        .with_client_tls(config.client.tls.build()?);

    let stats = config.stats.build(cluster.origin.clone());
    let cluster = cluster.with_stats(stats);

    // Spawn the health monitor before `config.web` is moved into the server.
    let health = config.web.health.build();

    // Create a web server too. mTLS for HTTPS is opt-in via `--web-https-root`.
    let web = Web::new(
        WebState {
            auth: auth.clone(),
            cluster: cluster.clone(),
            tls_info: server.tls_info(),
            conn_id: Default::default(),
            health,
        },
        config.web,
    );


    let state = RelayState {
        auth,
        cluster,
        tls_info: server.tls_info(),
    };

    Ok(RelayServer { state, server, web })
}

pub async fn start_moq_udp_only(
    relay: RelayServer,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let cluster_for_serve = relay.state.cluster.clone();
    let cluster_for_run = relay.state.cluster;

    let auth = relay.state.auth.clone();
    let server = relay.server;
    let web = relay.web;
    tokio::select! {
        _ = shutdown.cancelled() => {
            log::info!("handle relay shutdown requested");
            Ok(())
        }
        Err(err) = cluster_for_run.run() => Err(err).context("cluster failed"),
        Err(err) = web.run() => return Err(err).context("web server failed"),
        Err(err) = serve(server, cluster_for_serve, auth, shutdown.clone()) => Err(err).context("server failed"),
        else => Ok(()),
    }
}

async fn serve(
    mut server: moq_native::Server,
    cluster: moq_relay::Cluster,
    auth: moq_relay::Auth,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let mut conn_id = 0;
    let mut tasks = tokio::task::JoinSet::new();

    log::info!(
        "relay server listening on {}",
        server.local_addr()?.to_string()
    );

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                log::info!("relay accept loop stopping");
                tasks.abort_all();

                while let Some(res) = tasks.join_next().await {
                    if let Err(err) = res {
                        if !err.is_cancelled() {
                            tracing::warn!(%err, "connection task failed");
                        }
                    }
                }

                return Ok(());
            }

            request = server.accept() => {
                let Some(request) = request else {
                    anyhow::bail!("stopped accepting connections");
                };

                let conn = moq_relay::Connection {
                    id: conn_id,
                    request,
                    cluster: cluster.clone(),
                    auth: auth.clone(),
                };

                conn_id += 1;

                tasks.spawn(async move {
                    if let Err(err) = conn.run().await {
                        tracing::warn!(%err, "connection closed");
                    }
                });
            }
        }
    }
}
