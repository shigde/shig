use crate::db::user_roles::Role;
use crate::db::{users::admin, DbPool};
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use actix_web::{get, patch, web, HttpResponse};
use serde::Deserialize;

#[get("/monitoring")]
pub async fn monitoring(session: web::ReqData<Session>) -> HttpResponse {
    if session.principal.user_role_id != Role::Admin.val() {
        return HttpResponse::Forbidden().finish();
    }
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(crate::sfu::metrics::snapshot())
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

#[derive(Deserialize)]
pub struct UserQuery {
    #[serde(default)]
    q: String,
    #[serde(default)]
    offset: i64,
}

fn require_admin(session: &Session) -> Result<(), ApiError> {
    if session.principal.user_role_id != Role::Admin.val() {
        return Err(ApiError::Forbidden {
            error_message: "Administrator access required".into(),
        });
    }
    Ok(())
}

#[get("/users")]
pub async fn list_users(
    session: web::ReqData<Session>,
    pool: web::Data<DbPool>,
    query: web::Query<UserQuery>,
) -> Result<HttpResponse, ApiError> {
    require_admin(&session)?;
    if query.offset < 0 || query.q.len() > 200 {
        return Err(ApiError::BadRequest {
            error_message: "Invalid search or offset".into(),
        });
    }
    let mut conn = pool.get()?;
    let page = admin::list(&mut conn, query.q.trim(), query.offset)?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(Body::new("ok", page)))
}

#[derive(Deserialize)]
pub struct ActiveUpdate {
    active: bool,
}

#[patch("/users/{uuid}/active")]
pub async fn set_user_active(
    session: web::ReqData<Session>,
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    update: web::Json<ActiveUpdate>,
) -> Result<HttpResponse, ApiError> {
    require_admin(&session)?;
    let uuid = path.into_inner();
    if !update.active && uuid == session.principal.user_uuid {
        return Err(ApiError::BadRequest {
            error_message: "You cannot block your own account".into(),
        });
    }
    let mut conn = pool.get()?;
    if !admin::set_active(&mut conn, &uuid, update.active)? {
        return Err(ApiError::NotFound {
            error_message: "User not found".into(),
        });
    }
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(Body::new("ok", update.active)))
}

#[cfg(test)]
mod user_tests {
    use super::*;
    use actix_web::{test, App, HttpMessage};
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::PgConnection;

    fn session(role: i32) -> Session {
        serde_json::from_value(serde_json::json!({"principal": {
            "id": 1, "name": "admin", "email": "admin@example.com", "user_uuid": "self",
            "user_role_id": role, "user_actor_id": 1, "channel_id": 1,
            "channel_uuid": "channel", "channel_actor_id": 1, "avatar": null
        }}))
        .unwrap()
    }

    #[actix_web::test]
    async fn user_management_rejects_non_admins_and_self_blocking() {
        // No database connections are opened: every tested request must be rejected first.
        let pool = Pool::builder()
            .min_idle(Some(0))
            .max_size(1)
            .build(ConnectionManager::<PgConnection>::new("postgres://unused"))
            .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(list_users)
                .service(set_user_active)
                .service(set_user_role),
        )
        .await;
        for role in [Role::User.val(), Role::Guest.val()] {
            let request = test::TestRequest::get().uri("/users").to_request();
            request.extensions_mut().insert(session(role));
            assert_eq!(test::call_service(&app, request).await.status(), 403);
            let request = test::TestRequest::patch()
                .uri("/users/other/active")
                .set_json(serde_json::json!({"active": false}))
                .to_request();
            request.extensions_mut().insert(session(role));
            assert_eq!(test::call_service(&app, request).await.status(), 403);
        }
        for (caller, target, role, status) in [
            (Role::User.val(), "other", 1, 403),
            (Role::Guest.val(), "other", 2, 403),
            (Role::Admin.val(), "self", 2, 400),
            (Role::Admin.val(), "other", 3, 400),
        ] {
            let request = test::TestRequest::patch()
                .uri(&format!("/users/{target}/role"))
                .set_json(serde_json::json!({"role": role}))
                .to_request();
            request.extensions_mut().insert(session(caller));
            assert_eq!(test::call_service(&app, request).await.status(), status);
        }
        let request = test::TestRequest::patch()
            .uri("/users/self/active")
            .set_json(serde_json::json!({"active": false}))
            .to_request();
        request.extensions_mut().insert(session(Role::Admin.val()));
        assert_eq!(test::call_service(&app, request).await.status(), 400);
        let request = test::TestRequest::get()
            .uri("/users?offset=-1")
            .to_request();
        request.extensions_mut().insert(session(Role::Admin.val()));
        assert_eq!(test::call_service(&app, request).await.status(), 400);
    }
}

#[derive(Deserialize)]
pub struct RoleUpdate {
    role: i32,
}

#[patch("/users/{uuid}/role")]
pub async fn set_user_role(
    session: web::ReqData<Session>,
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    update: web::Json<RoleUpdate>,
) -> Result<HttpResponse, ApiError> {
    require_admin(&session)?;
    let uuid = path.into_inner();
    if uuid == session.principal.user_uuid
        || ![Role::Admin.val(), Role::User.val()].contains(&update.role)
    {
        return Err(ApiError::BadRequest {
            error_message: "Only User/Admin roles on other accounts can be changed".into(),
        });
    }
    let mut conn = pool.get()?;
    match admin::set_role(&mut conn, session.principal.id, &uuid, update.role)? {
        admin::RoleChange::Updated => Ok(HttpResponse::Ok()
            .insert_header(("Cache-Control", "no-store"))
            .json(Body::new("ok", update.role))),
        admin::RoleChange::NotFound => Err(ApiError::NotFound {
            error_message: "User not found".into(),
        }),
        admin::RoleChange::Forbidden => Err(ApiError::Forbidden {
            error_message: "Administrator access required".into(),
        }),
        admin::RoleChange::UnsupportedRole => Err(ApiError::Conflict {
            error_message: "Only User/Admin accounts can be changed".into(),
        }),
    }
}
