mod error;

use crate::api;
use crate::db::DbConfig;
use crate::federation::FederationConfig;
use crate::files::FilesConfig;
use crate::models::auth::jwt::JWTConfig;
use crate::models::mail::config::MailConfig;
use crate::relay::config::RelayConfig;
use crate::relay::state::RelayState;
use crate::server::error::ServerResult;
use crate::sfu::config::SfuConfig;
use crate::sfu::Sfu;
use actix::Addr;
use actix_web::dev::Server;
use actix_web::{get, web, App, HttpRequest, HttpServer, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[get("/")]
async fn index(_req: HttpRequest) -> impl Responder {
    "Welcome on Shig Server!"
}

// Top level struct to hold the TOML data.
#[derive(Deserialize, Clone, Debug)]
pub struct ConfigFile {
    pub server: ServerConfig,
    pub sfu: SfuConfig,
    pub files: FilesConfig,
    pub federation: FederationConfig,
    pub database: DbConfig,
    pub jwt: JWTConfig,
    pub mail: MailConfig,

    #[serde(default)]
    pub relay: RelayConfig,
}

// Config struct holds to data from the `[config]` section.
#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls: TlsConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TlsConfig {
    pub enabled: bool,
    cert: String,
    key: String,
}

impl ServerConfig {
    fn url(self) -> String {
        format!(
            "{}://localhost:8080",
            if self.tls.enabled { "https" } else { "http" }
        )
    }
}

pub fn start(
    cfg: ConfigFile,
    sfu_addr: Addr<Sfu>,
    pool: Pool<ConnectionManager<PgConnection>>,
    relay_state: RelayState,
) -> ServerResult<Server> {
    // create static file dir if not exists
    let htdocs = cfg.files.htdocs.as_str();
    if !Path::new(htdocs).exists() {
        let avatar = format!("{htdocs}/avatar");
        let banner = format!("{htdocs}/banner");
        let thumbnail = format!("{htdocs}/thumbnail");
        fs::create_dir(htdocs)?;
        fs::create_dir(avatar)?;
        fs::create_dir(banner)?;
        fs::create_dir(thumbnail)?;
    }

    let svs = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(sfu_addr.clone()))
            .app_data(web::Data::new(relay_state.clone()))
            .app_data(web::Data::new(cfg.federation.clone()))
            .app_data(web::Data::new(cfg.jwt.clone()))
            .app_data(web::Data::new(cfg.mail.clone()))
            .app_data(web::Data::new(cfg.files.clone()))
            .wrap(crate::middleware::auth::Authentication)
            .wrap(crate::middleware::req::LoggingMiddleware)
            .configure(api::config_services)
    });

    let url = cfg.server.clone().url();
    let server = if cfg.server.tls.enabled {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
        builder.set_private_key_file(cfg.server.tls.key, SslFiletype::PEM)?;
        builder.set_certificate_chain_file(cfg.server.tls.cert)?;
        svs.bind_openssl((cfg.server.host, cfg.server.port), builder)?
            .run()
    } else {
        svs.bind((cfg.server.host.clone(), cfg.server.port))?.run()
    };

    log::info!("web server start listening on: {}", url);

    Ok(server)
}
