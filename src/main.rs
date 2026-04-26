extern crate core;

mod api;
mod db;
mod federation;
mod files;
mod middleware;
mod models;
mod server;
mod sfu;
mod util;
mod relay;
mod worker;

use crate::db::fixtures::insert_fixtures;
use crate::db::{build_pool, run_migrations};
use crate::sfu::{Sfu, Shutdown};
use actix::Actor;
use clap::ArgAction;
use clap::Parser;
use server::ConfigFile;
use std::fs;
use std::process::exit;
use tokio::signal;

use crate::relay::{new_relay_server, start_moq_udp_only};

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

        // Start the SFU server
        let sfu = Sfu::new(server_cfg.sfu.clone(), pool.clone());
        let sfu_addr = sfu.start();

        // Shutdown-Signal
        let sfu_addr_cp = sfu_addr.clone();
        let shutdown = async {
            signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
            log::info!("Shutdown signal received!");

            sfu_addr_cp.do_send(Shutdown {});

            // Currently running requests are allowed to complete
            // Then stops the entire Actix system
            actix::System::current().stop();
        };

        // relay server
        let relay_server = new_relay_server(server_cfg.relay.clone()).await.unwrap_or_else(|e| {
            log::error!("failed to init relay server: {:?}", e);
            exit(1);
        });

        let relay_state = relay_server.state.clone();

        // UDP/MoQ-Server
        let moq_task = async move {
            if let Err(e) = start_moq_udp_only(relay_server).await {
                log::error!("moq udp server failed: {:?}", e);
            }
        };

        // HTTP/WS-Server
        log::info!("starting actix web server on tcp/8080");
        let web_server = match server::start(server_cfg.clone(), sfu_addr, pool.clone(), relay_state) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to start server: {}", e);
                exit(1);
            }
        };

        tokio::select! {
            _ = web_server => {
                log::info!("Actix web server was closed");
            },
            _ = moq_task => {
                log::info!("MoQ UDP server was closed");
            },
            _ = shutdown => {
                log::info!("Shutdown done!");
            }
        }
    });
}
