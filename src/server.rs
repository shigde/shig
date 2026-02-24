mod error;

use crate::api;
use crate::db::DbConfig;
use crate::federation::FederationConfig;
use crate::files::FilesConfig;
use crate::models::auth::jwt::JWTConfig;
use crate::models::mail::config::MailConfig;
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
}

// Config struct holds to data from the `[config]` section.
#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    host: String,
    port: u16,
    tls: TlsConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TlsConfig {
    enabled: bool,
    cert: String,
    key: String,
}

pub fn start(
    cfg: ConfigFile,
    sfu_addr: Addr<Sfu>,
    pool: Pool<ConnectionManager<PgConnection>>,
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
            .app_data(web::Data::new(cfg.federation.clone()))
            .app_data(web::Data::new(cfg.jwt.clone()))
            .app_data(web::Data::new(cfg.mail.clone()))
            .app_data(web::Data::new(cfg.files.clone()))
            .wrap(crate::middleware::auth::Authentication)
            .wrap(crate::middleware::req::LoggingMiddleware)
            .configure(api::config_services)
    });

    let server = if cfg.server.tls.enabled {
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
    } else {
        log::info!(
            "web server start listening on: http://{}:{}/",
            cfg.server.host,
            cfg.server.port
        );
        svs.bind((cfg.server.host.clone(), cfg.server.port))?.run()
    };

    Ok(server)
}
