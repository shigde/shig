extern crate core;

mod api;
mod db;
mod federation;
mod files;
mod middleware;
mod models;
mod relay;
mod server;
mod sfu;
mod util;
mod worker;

use crate::db::fixtures::insert_fixtures;
use crate::db::{build_pool, run_migrations};
use crate::relay::{new_relay_server, start_moq_udp_only};
use crate::sfu::{Sfu, Shutdown};
use actix::Actor;
use clap::ArgAction;
use clap::Parser;
use server::ConfigFile;
use std::fs;
use std::process::exit;
use tokio::signal;
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(name = "Shig Server")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Turn debugging information on
    #[arg(short, long, action=ArgAction::SetFalse)]
    debug: bool,

    #[arg(short, long, default_value_t = String::from("config/default.toml"))]
    config: String,
}

fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let cli = Cli::parse();

    let filename = &cli.config[..];
    let contents = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(_) => {
            log::error!("could not read config file `{}`", filename);
            exit(1);
        }
    };

    let server_cfg: ConfigFile = match toml::from_str(&contents) {
        Ok(d) => d,
        Err(e) => {
            log::error!("unable to load data from `{}`, {}", filename, e);
            exit(1);
        }
    };

    actix::System::new().block_on(async {
        // Set up the connection pool
        let pool = build_pool(server_cfg.database.clone()).unwrap_or_else(|error| {
            log::error!("failed to create pool: {}", error);
            exit(1);
        });

        let mut conn = pool.get().unwrap_or_else(|error| {
            log::error!("failed to get pool connection: {}", error);
            exit(1);
        });

        if let Err(e) = run_migrations(&mut conn) {
            log::error!("failed to run migrations: {}", e);
            exit(1);
        }

        if let Err(e) = insert_fixtures(&mut conn, server_cfg.federation.clone()) {
            log::error!("failed to insert fixtures: {}", e);
            exit(1);
        }

        // relay server
        let relay_server = new_relay_server(server_cfg.relay.clone())
            .await
            .unwrap_or_else(|e| {
                log::error!("failed to init relay server: {:?}", e);
                exit(1);
            });
        let relay_state = relay_server.state.clone();

        // Start the SFU server
        let sfu = Sfu::new(
            server_cfg.sfu.clone(),
            pool.clone(),
            relay_server.state.clone(),
        );
        let sfu_addr = sfu.start();

        // Shutdown-Signal
        let sfu_addr_cp = sfu_addr.clone();
        let shutdown_token = CancellationToken::new();
        let shutdown = async {
            signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
            log::info!("Shutdown signal received!");

            // stop sfu
            sfu_addr_cp
                .send(Shutdown {})
                .await
                .expect("Failed shut down sfu");

            // stop relay
            shutdown_token.cancel();

            // Currently, running requests are allowed to complete
            // Then stops the entire Actix system
            actix::System::current().stop();
        };

        let relay_shutdown = shutdown_token.clone();
        // Start UDP/MoQ-Server
        let moq_task = async move {
            if let Err(e) = start_moq_udp_only(relay_server, relay_shutdown).await {
                log::error!("moq udp server failed: {:?}", e);
            }
        };

        // HTTP/WS-Server
        log::info!("starting actix web server on");
        let web_server =
            match server::start(server_cfg.clone(), sfu_addr, pool.clone(), relay_state) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to start server: {}", e);
                    exit(1);
                }
            };

        let ((), (), ()) = tokio::join!(
            async {
                let  _ = web_server.await;
                log::info!("Actix web server was closed");
            },
            async {
                moq_task.await;
                log::info!("MoQ udp server was closed");
            },
            async {
                shutdown.await;
                log::info!("shutdown signal received");
            }
        );

        log::info!("Shutdown done!");
    });
}
