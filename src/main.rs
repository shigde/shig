extern crate core;

mod api;
mod db;
mod server;
mod util;
mod federation;
mod middleware;
mod models;
mod files;
mod sfu;

use clap::ArgAction;
use clap::Parser;
use server::ConfigFile;
use std::fs;
use std::process::exit;
use actix::Actor;
use tokio::signal;
use crate::sfu::{Sfu, StopNow};

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

#[tokio::main]
async fn main() {
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
        Err(_) => {
            log::error!("unable to load data from `{}`", filename);
            exit(1);
        }
    };

    // Start the SFU server
    let sfu = Sfu::new(server_cfg.sfu.clone());
    let sfu_addr = sfu.start();

    // Shutdown-Signal vorbereiten
    let sfu_addr_cp = sfu_addr.clone();
    let shutdown = async {
        signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        log::info!("Shutdown signal received!");
        
        sfu_addr_cp.do_send(StopNow{});

        // Currently running requests are allowed to complete
        // Then stops the entire Actix system
        actix::System::current().stop();
    };

    // Start web server
    log::info!("starting web server");
    let server = server::start(server_cfg, sfu_addr);
    
    tokio::select! {
        _ = server => {
            log::info!("Server was closed");
        },
        _ = shutdown => {
            log::info!("Shutdown done!");       
        }
    }
}
