//! Sequential, allowlisted reads. A resource failure never changes the batch HTTP status.
use crate::{
    db::{inbox::{read as inbox_read, Inbox}, DbPool},
    models::{auth::session::Session, error::ApiError, inbox::{Counts, Page}},
    relay::state::RelayState,
    sfu::Sfu,
};
use actix_web::{post, web, HttpRequest, HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    requests: Vec<Item>,
}
#[derive(Deserialize)]
pub struct Item {
    id: String,
    resource: String,
    #[serde(flatten)]
    params: HashMap<String, Value>,
}
#[derive(Serialize)]
pub struct ResultItem {
    id: String,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ResourceError>,
}
#[derive(Serialize)]
pub struct ResourceError {
    code: &'static str,
    message: String,
}
impl ResultItem {
    fn failure(id: String, status: u16, code: &'static str, message: &str) -> Self {
        Self {
            id,
            status,
            data: None,
            error: Some(ResourceError {
                code,
                message: message.into(),
            }),
        }
    }
    fn from_error(id: String, error: ApiError) -> Self {
        let status = error.status_code().as_u16();
        let (code, message) = match status {
            400 => ("INVALID_ARGUMENT", "Invalid resource parameters"),
            401 => ("UNAUTHORIZED", "Authentication required for resource"),
            403 => ("FORBIDDEN", "Access to resource denied"),
            404 => ("NOT_FOUND", "Resource not found"),
            409 => ("CONFLICT", "Resource state conflict"),
            503 => ("UNAVAILABLE", "Resource temporarily unavailable"),
            _ => ("INTERNAL_ERROR", "Resource could not be loaded"),
        };
        Self::failure(id, status, code, message)
    }
}
fn invalid() -> ApiError {
    ApiError::BadRequest {
        error_message: "Invalid resource parameters".into(),
    }
}
fn param<'a>(item: &'a Item, key: &str) -> Result<&'a str, ApiError> {
    let value = item
        .params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    if value.is_empty() || value.len() > 255 || uuid::Uuid::parse_str(value).is_err() {
        return Err(invalid());
    }
    Ok(value)
}
fn keys(item: &Item, allowed: &[&str]) -> Result<(), ApiError> {
    if item
        .params
        .keys()
        .any(|key| !allowed.contains(&key.as_str()))
    {
        return Err(invalid());
    }
    Ok(())
}
fn validate(batch: &Batch) -> Result<(), ApiError> {
    let mut ids = HashSet::new();
    if batch.requests.is_empty() || batch.requests.len() > 32 {
        return Err(invalid());
    }
    for item in &batch.requests {
        if item.id.is_empty()
            || item.id.len() > 128
            || item.resource.len() > 64
            || !ids.insert(&item.id)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

#[post("/poll")]
pub async fn poll(
    request: HttpRequest,
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
    batch: web::Json<Batch>,
) -> Result<HttpResponse, ApiError> {
    validate(&batch)?;
    let mut results = Vec::with_capacity(batch.requests.len());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    for item in &batch.requests {
        if ![
            "messages-counts",
            "notifications-counts",
            "messages",
            "notifications",
            "monitoring",
            "stream-status",
            "host-presence",
        ]
        .contains(&item.resource.as_str())
        {
            results.push(ResultItem::failure(
                item.id.clone(),
                400,
                "UNKNOWN_RESOURCE",
                "Unknown polling resource",
            ));
            continue;
        }
        let result = tokio::time::timeout_at(
            deadline.min(tokio::time::Instant::now() + Duration::from_secs(2)),
            dispatch(item, &request, &pool, &session),
        )
        .await;
        results.push(match result {
            Ok(Ok(data)) => ResultItem {
                id: item.id.clone(),
                status: 200,
                data: Some(data),
                error: None,
            },
            Ok(Err(error)) => ResultItem::from_error(item.id.clone(), error),
            Err(_) => ResultItem::failure(item.id.clone(), 504, "TIMEOUT", "Resource timed out"),
        });
    }
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(json!({"results": results})))
}

async fn dispatch(
    item: &Item,
    request: &HttpRequest,
    pool: &web::Data<DbPool>,
    session: &Session,
) -> Result<Value, ApiError> {
    match item.resource.as_str() {
        "monitoring" => {
            keys(item, &[])?;
            if session.principal.user_role_id != crate::db::user_roles::Role::Admin.val() {
                return Err(ApiError::Forbidden {
                    error_message: "Administrator access required".into(),
                });
            }
            serialize(crate::sfu::metrics::snapshot())
        }
        "messages-counts" | "notifications-counts" | "messages" | "notifications" => {
            let counts = item.resource.ends_with("-counts");
            keys(item, if counts { &[] } else { &["offset"] })?;
            let offset = match item.params.get("offset") {
                Some(value) => value
                    .as_i64()
                    .filter(|offset| *offset >= 0)
                    .ok_or_else(invalid)?,
                None => 0,
            };
            let inbox = if item.resource.starts_with("messages") {
                Inbox::Messages
            } else {
                Inbox::Notifications
            };
            let user = session.principal.id;
            let pool = pool.clone();
            web::block(move || {
                let mut conn = pool.get_timeout(Duration::from_millis(250)).map_err(|_| {
                    ApiError::ServiceUnavailable {
                        error_message: "Database busy".into(),
                    }
                })?;
                // Keep database reads bounded even though dispatch is sequential.
                use diesel::{connection::SimpleConnection, Connection};
                conn.transaction::<_, ApiError, _>(|conn| {
                    conn.batch_execute("SET LOCAL statement_timeout = '1500ms'")?;
                    if counts {
                        serialize(Counts::from(inbox_read::counts(conn, inbox, user)?))
                    } else {
                        serialize(Page::from(inbox_read::list(conn, inbox, user, offset)?))
                    }
                })
            })
            .await
            .map_err(|_| ApiError::ServiceUnavailable {
                error_message: "Database worker unavailable".into(),
            })?
        }
        "stream-status" => {
            keys(item, &["channelUuid"])?;
            let channel = param(item, "channelUuid")?;
            let state = request.app_data::<web::Data<RelayState>>().ok_or_else(|| {
                ApiError::ServiceUnavailable {
                    error_message: "Relay unavailable".into(),
                }
            })?;
            let value = crate::api::relay::announcement::announced_prefix(
                state,
                request,
                &format!("live/{channel}"),
            )
            .await
            .map_err(|_| ApiError::Forbidden {
                error_message: "Stream access denied".into(),
            })?;
            Ok(json!(value))
        }
        "host-presence" => {
            keys(item, &["channelUuid", "streamUuid"])?;
            let channel = param(item, "channelUuid")?;
            let stream = param(item, "streamUuid")?;
            let sfu = request
                .app_data::<web::Data<actix::Addr<Sfu>>>()
                .ok_or_else(|| ApiError::ServiceUnavailable {
                    error_message: "SFU unavailable".into(),
                })?;
            let present = crate::models::lobby::is_host_present(
                pool,
                channel.into(),
                stream.into(),
                session.principal.clone(),
                sfu.clone(),
            )
            .await?;
            Ok(json!({"data": present}))
        }
        _ => Err(invalid()),
    }
}
fn serialize(value: impl Serialize) -> Result<Value, ApiError> {
    serde_json::to_value(value).map_err(|_| ApiError::InternalServerError {
        error_message: "Serialization failed".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App, HttpMessage};
    use diesel::{
        r2d2::{ConnectionManager, Pool},
        PgConnection,
    };

    fn session(role: i32) -> Session {
        serde_json::from_value(json!({"principal": {
            "id": 1, "name": "test", "email": "test@example.com", "user_uuid": "test",
            "user_role_id": role, "user_actor_id": 1, "channel_id": 1,
            "channel_uuid": "test", "channel_actor_id": 1, "avatar": null
        }}))
        .unwrap()
    }
    fn pool() -> DbPool {
        Pool::builder()
            .min_idle(Some(0))
            .max_size(1)
            .build(ConnectionManager::<PgConnection>::new("postgres://unused"))
            .unwrap()
    }
    #[actix_web::test]
    async fn resource_errors_are_isolated_and_batches_stay_successful() {
        let app =
            test::init_service(App::new().app_data(web::Data::new(pool())).service(poll)).await;
        for role in [1, 2] {
            let request = test::TestRequest::post()
                .uri("/poll")
                .set_json(json!({"requests": [
                    {"id":"monitor","resource":"monitoring"},
                    {"id":"unknown","resource":"anything"},
                    {"id":"list","resource":"messages","offset":-1},
                    {"id":"foreign","resource":"notifications-counts","recipientId":2},
                    {"id":"monitor-again","resource":"monitoring"}
                ]}))
                .to_request();
            request.extensions_mut().insert(session(role));
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers().get("Cache-Control").unwrap(), "no-store");
            let body: Value = test::read_body_json(response).await;
            assert_eq!(
                body["results"][0]["status"],
                if role == 1 { 200 } else { 403 }
            );
            assert_eq!(body["results"][1]["error"]["code"], "UNKNOWN_RESOURCE");
            assert_eq!(body["results"][2]["error"]["code"], "INVALID_ARGUMENT");
            assert_eq!(body["results"][3]["status"], 400);
            assert_eq!(
                body["results"][4]["status"],
                if role == 1 { 200 } else { 403 }
            );
            if role == 1 {
                assert!(body["results"][0]["data"]["active_lobbies"].is_number());
            }
        }
    }
    #[actix_web::test]
    async fn invalid_envelopes_are_rejected_and_auth_is_required() {
        let app =
            test::init_service(App::new().app_data(web::Data::new(pool())).service(poll)).await;
        for payload in [
            json!({"requests":[]}),
            json!({"requests":[{"id":"same","resource":"monitoring"},{"id":"same","resource":"monitoring"}]}),
            json!({"requests":vec![json!({"id":"x","resource":"monitoring"});33]}),
        ] {
            let request = test::TestRequest::post()
                .uri("/poll")
                .set_json(payload)
                .to_request();
            request.extensions_mut().insert(session(1));
            assert_eq!(test::call_service(&app, request).await.status(), 400);
        }
        let app = test::init_service(
            App::new()
                .wrap(crate::middleware::auth::Authentication)
                .service(poll),
        )
        .await;
        let request = test::TestRequest::post()
            .uri("/poll")
            .set_json(json!({"requests":[{"id":"x","resource":"monitoring"}]}))
            .to_request();
        assert_eq!(test::call_service(&app, request).await.status(), 401);
    }
}
