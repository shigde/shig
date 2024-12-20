mod error;

use crate::db::{build_pool, DbConfig};
use crate::server::error::ServerResult;

use actix_web::{get, web, App, HttpRequest, HttpServer, Responder};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde_derive::Deserialize;
use crate::server;

#[get("/")]
async fn index(_req: HttpRequest) -> impl Responder {
    "Welcome on Shig Server!"
}

// Top level struct to hold the TOML data.
#[derive(Deserialize)]
pub struct ConfigFile {
    server: ServerConfig,
    database: DbConfig,
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

pub async fn start(cfg: ConfigFile) -> ServerResult<()> {
    // Set up the connection pool
    let pool = build_pool(cfg.database.name.clone())?;
    let svs = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(index)
    });

    if cfg.server.tls.enabled {
        log::info!(
            "web server start listening on: https://{}:{}/",
            cfg.server.host,
            cfg.server.port
        );
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
        builder.set_private_key_file(cfg.server.tls.key, SslFiletype::PEM)?;
        builder.set_certificate_chain_file(cfg.server.tls.cert)?;
        svs.bind_openssl((cfg.server.host, cfg.server.port), builder)?
            .run()
            .await
            .map_err(server::error::ServerError::from)
    } else {
        log::info!(
            "web server start listening on: http://{}:{}/",
            cfg.server.host,
            cfg.server.port
        );
        svs.bind((cfg.server.host.clone(), cfg.server.port))?
            .run()
            .await
            .map_err(server::error::ServerError::from)
    }
    // .map_err(anyhow::Error::from)
}
