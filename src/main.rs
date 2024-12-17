use actix_web::{get, App, HttpRequest, HttpServer, Responder};
use clap::ArgAction;
use clap::Parser;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde_derive::Deserialize;
use std::fs;
use std::process::exit;

#[get("/")]
async fn index(_req: HttpRequest) -> impl Responder {
    "Welcome on Shig Server!"
}

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

// Top level struct to hold the TOML data.
#[derive(Deserialize)]
struct ConfigFile {
    server: ServerConfig,
}

// Config struct holds to data from the `[config]` section.
#[derive(Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
    tls: TlsConfig,
}

#[derive(Deserialize)]
struct TlsConfig {
    enabled: bool,
    cert: String,
    key: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let filename = &cli.config[..];
    let contents = match fs::read_to_string(filename) {
        // If successful return the files text as `contents`.
        // `c` is a local variable.
        Ok(c) => c,
        // Handle the `error` case.
        Err(_) => {
            // Write `msg` to `stderr`.
            eprintln!("Could not read config file `{}`", filename);
            // Exit the program with exit code `1`.
            exit(1);
        }
    };

    // Use a `match` block to return the
    // file `contents` as a `Data struct: Ok(d)`
    // or handle any `errors: Err(_)`.
    let server_cfg: ConfigFile = match toml::from_str(&contents) {
        // If successful, return data as `Data` struct.
        // `d` is a local variable.
        Ok(d) => d,
        // Handle the `error` case.
        Err(_) => {
            // Write `msg` to `stderr`.
            eprintln!("Unable to load data from `{}`", filename);
            // Exit the program with exit code `1`.
            exit(1);
        }
    };

    if server_cfg.server.tls.enabled {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file(server_cfg.server.tls.key, SslFiletype::PEM)
            .unwrap();
        builder
            .set_certificate_chain_file(server_cfg.server.tls.cert)
            .unwrap();

        println!("Server running: https://{}:{}/", server_cfg.server.host, server_cfg.server.port);
        HttpServer::new(|| App::new().service(index))
            .bind_openssl((server_cfg.server.host, server_cfg.server.port), builder)?
            .run()
            .await
    } else {
        println!("Server running: http://{}:{}/", server_cfg.server.host, server_cfg.server.port);
        HttpServer::new(|| App::new().service(index))
            .bind((server_cfg.server.host, server_cfg.server.port))?
            .run()
            .await
    }
}
