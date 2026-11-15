use crate::db::user_roles::Role;
use crate::models::auth::session::Session;
use actix_web::{get, web, HttpResponse};

#[get("/monitoring")]
pub async fn monitoring(session: web::ReqData<Session>) -> HttpResponse {
    if session.principal.user_role_id != Role::Admin.val() {
        return HttpResponse::Forbidden().finish();
    }
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(crate::metrics::snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App, HttpMessage};

    #[actix_web::test]
    async fn monitoring_requires_admin_role() {
        let app = test::init_service(App::new().service(monitoring)).await;
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
            let request = test::TestRequest::get().uri("/monitoring").to_request();
            request.extensions_mut().insert(session);
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status().as_u16(), expected);
            if expected == 200 {
                assert_eq!(response.headers().get("Cache-Control").unwrap(), "no-store");
                let body: serde_json::Value = test::read_body_json(response).await;
                assert!(body["active_lobbies"].is_number());
                assert!(body["active_peers"].is_number());
                assert!(body["lobbies"].is_array());
            }
        }
    }
}
