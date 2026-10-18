use super::*;
use crate::db::{run_migrations, users::read::find_user_by_id};
use crate::models::auth::jwt::{JWTConfig, JWT};
use actix_web::{http::StatusCode, test, App};
use diesel::connection::SimpleConnection;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Pool};

#[derive(Debug)]
struct SearchPath(String);

impl CustomizeConnection<PgConnection, diesel::r2d2::Error> for SearchPath {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), diesel::r2d2::Error> {
        conn.batch_execute(&format!("SET search_path TO {}, public", self.0))
            .map_err(diesel::r2d2::Error::QueryError)
    }
}

struct Fixture {
    admin: PgConnection,
    schema: String,
    pool: DbPool,
    jwt: JWTConfig,
    token: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.admin
            .batch_execute(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .unwrap();
    }
}

impl Fixture {
    fn new() -> Self {
        let url = std::env::var("SHIG_MIGRATION_TEST_DATABASE_URL")
            .expect("dedicated test database URL required");
        let mut admin = PgConnection::establish(&url).unwrap();
        let schema = format!("visibility_{}", uuid::Uuid::new_v4().simple());
        admin
            .batch_execute(&format!("CREATE SCHEMA {schema}"))
            .unwrap();
        let pool = Pool::builder()
            .max_size(4)
            .connection_customizer(Box::new(SearchPath(schema.clone())))
            .build(ConnectionManager::<PgConnection>::new(url))
            .unwrap();
        let mut conn = pool.get().unwrap();
        run_migrations(&mut *conn).unwrap();
        conn.batch_execute(
            "INSERT INTO actors (preferred_username, actor_type, actor_iri, created_at) VALUES
                ('alice', 'Person', 'https://example.test/alice', now()),
                ('alice-channel', 'Group', 'https://example.test/channel-a', now()),
                ('bob', 'Person', 'https://example.test/bob', now()),
                ('bob-channel', 'Group', 'https://example.test/channel-b', now());
             INSERT INTO users (name, email, user_uuid, user_role_id, password, active, actor_id, created_at) VALUES
                ('alice', 'alice@example.test', 'alice', 2, 'unused', true, 1, now()),
                ('bob', 'bob@example.test', 'bob', 2, 'unused', true, 3, now());
             INSERT INTO channels (uuid, user_id, actor_id, name, created_at) VALUES
                ('channel-a', 1, 2, 'channel-a', now()), ('channel-b', 2, 4, 'channel-b', now());
             INSERT INTO streams (uuid, user_id, channel_id, title, date, licence, is_public, created_at) VALUES
                ('public', 1, 1, 'Public', now(), 1, true, now()),
                ('private', 1, 1, 'Private', now(), 1, false, now()),
                ('other-private', 2, 2, 'Other', now(), 1, false, now());"
        ).unwrap();
        let jwt = JWTConfig {
            auth_token_key: "visibility-test-key".into(),
            refresh_token_key: "unused".into(),
            session_live_time: 3600,
        };
        let user = find_user_by_id(&mut conn, 2).unwrap();
        let token = JWT::generate_token(&user, &web::Data::new(jwt.clone())).unwrap();
        drop(conn);
        Self {
            admin,
            schema,
            pool,
            jwt,
            token,
        }
    }
}

#[actix_web::test]
#[ignore = "Requires SHIG_MIGRATION_TEST_DATABASE_URL pointing to a dedicated PostgreSQL test database"]
async fn visibility_covers_public_apis_and_direct_relay_grants() {
    let fixture = Fixture::new();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(fixture.pool.clone()))
            .app_data(web::Data::new(fixture.jwt.clone()))
            .wrap(crate::middleware::auth::Authentication)
            .service(
                web::scope("/api/pub/stream-preview")
                    .service(crate::api::user::stream_preview::get_stream_preview_list)
                    .service(crate::api::user::stream_preview::get_stream_preview),
            )
            .service(
                web::scope("/api/pub/channel")
                    .service(crate::api::user::stream_preview::get_channel_stream_preview_list),
            ),
    )
    .await;

    for (path, expected) in [
        ("/api/pub/stream-preview", 1),
        ("/api/pub/channel/channel-a/stream-preview", 1),
    ] {
        let response =
            test::call_service(&app, test::TestRequest::get().uri(path).to_request()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(response).await;
        assert_eq!(body["data"].as_array().unwrap().len(), expected);
    }
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/pub/stream-preview/private")
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/pub/stream-preview/private")
            .insert_header(("Authorization", format!("Bearer {}", fixture.token)))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/pub/stream-preview")
            .insert_header(("Authorization", "Bearer invalid"))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let mut request = Request::new("test", moq_auth::Transport::Quic, "live/channel-a");
    let anonymous = decide(&request, &fixture.pool, &fixture.jwt).unwrap();
    assert!(anonymous.subscribe.matches("public"));
    assert!(!anonymous.subscribe.matches("private"));
    assert!(anonymous.publish.is_empty());
    request.query = Some(format!("jwt={}", fixture.token));
    let logged_in = decide(&request, &fixture.pool, &fixture.jwt).unwrap();
    assert!(logged_in.subscribe.matches("private"));
    assert!(!logged_in.subscribe.matches("other-private"));
    assert!(logged_in.publish.is_empty());
    // An authenticated account needs no invitation to view an account-only stream.
    let mut conn = fixture.pool.get().unwrap();
    assert!(!crate::db::stream_friends::read::is_stream_friend(&mut conn, 2, 2).unwrap());
    drop(conn);
    request.query = None;
    request.path = "live/channel-a/private".into();
    assert!(decide(&request, &fixture.pool, &fixture.jwt).is_err());
    request.query = Some("jwt=invalid".into());
    assert!(decide(&request, &fixture.pool, &fixture.jwt).is_err());

    let (auth, admissions) = moq_relay::auth::Auth::embedded("test");
    let serving = authorize_relay(admissions, fixture.pool.clone(), fixture.jwt.clone());
    let checking = async {
        let mut request = auth.request(moq_auth::Transport::Quic, "live/channel-a");
        let anonymous = auth.admit(request.clone()).await.unwrap();
        assert!(!anonymous.token().subscribe.matches("private"));
        request.query = Some(format!("jwt={}", fixture.token));
        let logged_in = auth.admit(request).await.unwrap();
        assert!(logged_in.token().subscribe.matches("private"));
        anonymous.close("done", Default::default());
        logged_in.close("done", Default::default());
    };
    tokio::select! {
        result = serving => panic!("authorization stopped unexpectedly: {result:?}"),
        _ = checking => {}
    }
}

#[actix_web::test]
#[ignore = "Requires SHIG_MIGRATION_TEST_DATABASE_URL pointing to a dedicated PostgreSQL test database"]
async fn deleting_stream_preserves_lobby_and_rejects_live_streams() {
    let fixture = Fixture::new();
    let mut conn = fixture.pool.get().unwrap();
    conn.batch_execute(
        "INSERT INTO stream_meta_data (stream_id, stream_key, url, protocol, latency_mode, created_at)
            VALUES (3, 'key', 'https://example.test/stream', 1, 1, now());
         INSERT INTO lobbies (uuid, user_id, channel_id, stream_id, is_open, created_at)
            VALUES ('lobby', 2, 2, 3, true, now());
         UPDATE streams SET is_live = true WHERE id = 3;"
    ).unwrap();
    drop(conn);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(fixture.pool.clone()))
            .app_data(web::Data::new(fixture.jwt.clone()))
            .wrap(crate::middleware::auth::Authentication)
            .service(
                web::scope("/api/user/stream").service(crate::api::user::stream::delete_stream),
            ),
    )
    .await;

    let response = test::call_service(
        &app,
        test::TestRequest::delete()
            .uri("/api/user/stream/other-private")
            .insert_header(("Authorization", format!("Bearer {}", fixture.token)))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body: serde_json::Value = test::read_body_json(response).await;
    assert!(body
        .to_string()
        .contains("Stop the stream before deleting it"));
    let mut conn = fixture.pool.get().unwrap();
    conn.batch_execute(
        "DO $$ BEGIN
        ASSERT EXISTS (SELECT 1 FROM streams WHERE id = 3 AND is_live);
        ASSERT EXISTS (SELECT 1 FROM lobbies WHERE uuid = 'lobby' AND stream_id = 3);
    END $$; UPDATE streams SET is_live = false WHERE id = 3;",
    )
    .unwrap();
    drop(conn);

    let response = test::call_service(
        &app,
        test::TestRequest::delete()
            .uri("/api/user/stream/other-private")
            .insert_header(("Authorization", format!("Bearer {}", fixture.token)))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut conn = fixture.pool.get().unwrap();
    conn.batch_execute("DO $$ BEGIN
        ASSERT NOT EXISTS (SELECT 1 FROM streams WHERE id = 3);
        ASSERT EXISTS (SELECT 1 FROM lobbies WHERE uuid = 'lobby' AND channel_id = 2 AND stream_id IS NULL AND is_open);
    END $$;").unwrap();
}
