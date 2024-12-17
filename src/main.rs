use actix_web::{get, App, HttpRequest, HttpServer, Responder};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use clap::ArgAction;
use clap::Parser;
use config::Config;

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

    #[arg(short, long, default_value_t = String::from("config/default.yml"))]
    config: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let cli = Cli::parse();

    let settings = Config::builder()
        // Add in `./Settings.toml`
        .add_source(config::File::with_name(&cli.config[..]))
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .unwrap();

    let cert: String = settings.get("server.tls.cert").unwrap();
    let key: String = settings.get("server.tls.key").unwrap();
    let port: u16 = settings.get("server.port").unwrap();
    let host: String = settings.get("server.host").unwrap();

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file(key, SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file(cert).unwrap();

    HttpServer::new(|| App::new().service(index))
        .bind_openssl((host, port), builder)?
        .run()
        .await
}

