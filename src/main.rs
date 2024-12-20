extern crate core;

mod api;
mod db;
mod server;

use clap::ArgAction;
use clap::Parser;
use server::ConfigFile;
use std::fs;
use std::process::exit;

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

    // Start web server
    log::info!("starting web server");
    match server::start(server_cfg).await {
        Ok(_) => {
            log::info!("Shig server stopped");
            return
        },
        Err(e) => {
            log::error!("web server fails: `{}`", e);
            exit(1);
        }
    };
}
