use super::{new_relay_server, start_moq_udp_only, RelayConfig};
use actix_web::{test as web_test, web, App};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[actix_web::test]
async fn default_config_uses_current_relay_schema() {
    let parsed: crate::server::ConfigFile =
        toml::from_str(include_str!("../../config/default.toml")).unwrap();
    let config = parsed.relay;
    assert_eq!(config.listen.bind.unwrap().to_string(), "[::]:4443");
    assert!(config.auth.public_grant().is_none());
    assert!(config.auth.url.is_none());
}

#[test]
fn embedded_publication_announcements_fingerprint_and_shutdown() {
    // The upstream debug startup future exceeds the default 2 MiB test stack.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| actix_rt::System::new().block_on(check_embedded_relay()))
        .unwrap()
        .join()
        .unwrap();
}

async fn check_embedded_relay() {
    let mut config = RelayConfig::default();
    config.listen.bind = Some("127.0.0.1:0".parse().unwrap());
    config.iroh.enabled = Some(false);
    config.auth.public.push("live/**".parse().unwrap());
    config.drain_timeout = Duration::ZERO;
    let server = new_relay_server(config).await.unwrap();
    let token = moq_relay::auth::Token::new(
        "live/channel",
        &moq_auth::Grant::new(
            ["**".parse().unwrap()].into_iter().collect(),
            Default::default(),
        ),
    );
    let origin = server.state.cluster.publisher(&token).unwrap();
    let broadcast = origin.publish("stream", Default::default()).unwrap();

    let app = web_test::init_service(
        App::new()
            .app_data(web::Data::new(server.state.clone()))
            .service(crate::api::relay::announcement::get_prefix)
            .service(crate::api::relay::tls::get_fingerprint),
    )
    .await;
    let request = web_test::TestRequest::get()
        .uri("/announced/live/channel")
        .to_request();
    let response = web_test::call_service(&app, request).await;
    assert!(response.status().is_success());
    assert_eq!(web_test::read_body(response).await.as_ref(), b"stream");

    let request = web_test::TestRequest::get()
        .uri("/announced/private")
        .to_request();
    let response = web_test::call_service(&app, request).await;
    assert!(response.status().is_client_error());

    let request = web_test::TestRequest::get()
        .uri("/certificate.sha256")
        .to_request();
    let response = web_test::call_service(&app, request).await;
    assert!(response.status().is_success());
    let fingerprint = web_test::read_body(response).await;
    assert_eq!(fingerprint.len(), 64);
    assert!(fingerprint.iter().all(u8::is_ascii_hexdigit));

    drop(broadcast);
    drop(origin);
    let ready = server.relay.ready();
    let shutdown = CancellationToken::new();
    let running = start_moq_udp_only(server, shutdown.clone());
    let stop = async {
        ready.wait().await.unwrap();
        shutdown.cancel();
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        let (result, ()) = tokio::join!(running, stop);
        result.unwrap();
    })
    .await
    .expect("relay did not stop");
}
