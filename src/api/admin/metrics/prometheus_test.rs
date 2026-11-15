use super::prometheus;
use crate::db::user_roles::Role;
use crate::models::auth::session::Session;
use actix_web::{test, App, HttpMessage};

#[actix_web::test]
async fn prometheus_requires_admin_role() {
    let app = test::init_service(App::new().service(prometheus)).await;
    for (role, expected) in [(Role::User.val(), 403), (Role::Admin.val(), 200)] {
        let session: Session = serde_json::from_value(serde_json::json!({
            "principal": {
                "id": 1, "name": "test", "email": "test@example.com",
                "user_uuid": "test", "user_role_id": role, "user_actor_id": 1,
                "channel_id": 1, "channel_uuid": "test", "channel_actor_id": 1,
                "avatar": null
            }
        }))
        .unwrap();
        let request = test::TestRequest::get().uri("/metrics").to_request();
        request.extensions_mut().insert(session);
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status().as_u16(), expected);
        if expected == 200 {
            assert_eq!(
                response.headers().get("Content-Type").unwrap(),
                "text/plain; version=0.0.4; charset=utf-8"
            );
        }
    }
}
