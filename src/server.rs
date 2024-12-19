use actix_web::{get, App, HttpRequest, HttpServer, Responder};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde_derive::Deserialize;

#[get("/")]
async fn index(_req: HttpRequest) -> impl Responder {
    "Welcome on Shig Server!"
}

// Top level struct to hold the TOML data.
#[derive(Deserialize)]
pub struct ConfigFile {
    server: ServerConfig,
}

// Config struct holds to data from the `[config]` section.
#[derive(Deserialize)]
pub struct ServerConfig {
    host: String,
    port: u16,
    tls: TlsConfig,
}

#[derive(Deserialize)]
pub struct TlsConfig {
    enabled: bool,
    cert: String,
    key: String,
}

pub async fn start(cfg: ConfigFile) -> std::io::Result<()> {
    if cfg.server.tls.enabled {
        start_https(cfg).await
    } else {
        start_http(cfg).await
    }
}

async fn start_http(cfg: ConfigFile) -> std::io::Result<()> {
    log::info!("web server start listening on: http://{}:{}/", cfg.server.host, cfg.server.port);
    HttpServer::new(|| App::new().service(index))
        .bind((cfg.server.host.clone(), cfg.server.port))?
        .run()
        .await
}

async fn start_https(cfg: ConfigFile) -> std::io::Result<()> {
    log::info!("web server start listening on: https://{}:{}/", cfg.server.host, cfg.server.port);

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
    builder.set_private_key_file(cfg.server.tls.key, SslFiletype::PEM)?;
    builder.set_certificate_chain_file(cfg.server.tls.cert)?;

    HttpServer::new(|| App::new().service(index))
        .bind_openssl((cfg.server.host, cfg.server.port), builder)?
        .run()
        .await
}
