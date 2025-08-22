use crate::db::DbPool;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::lobby::webrtc::egress::whep;
use crate::models::lobby::webrtc::ingress::whip;
use actix_web::{http::header, post, web, HttpRequest, HttpResponse};

#[post("/{channel_uuid}/stream/{stream_uuid}/whip")]
pub async fn whip_endpoint(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    sfu_addr: web::Data<actix::Addr<crate::sfu::Sfu>>,
    path: web::Path<(String, String)>,
    session: web::ReqData<Session>,
    body: String,
) -> Result<HttpResponse, ApiError> {
    let (channel_uuid, stream_uuid) = path.into_inner();
    let user = session.principal.clone();

    // Content-Type checking
    let content_type = req.headers().get(header::CONTENT_TYPE);
    if content_type != Some(&header::HeaderValue::from_static("application/sdp")) {
        return Ok(
            HttpResponse::UnsupportedMediaType().body("Expected Content-Type: application/sdp")
        );
    }

    // Accept checking
    if let Some(accept) = req.headers().get(header::ACCEPT) {
        if accept != "application/sdp" {
            return Ok(HttpResponse::NotAcceptable().body("Expected Accept: application/sdp"));
        }
    }

    let answer = whip(
        &pool,
        channel_uuid,
        stream_uuid,
        user,
        sfu_addr.clone(),
        body,
    )
    .await?;

    Ok(HttpResponse::Created()
        .content_type("application/sdp")
        .body(answer))
}

#[post("/{channel_uuid}/stream/{stream_uuid}/whep")]
pub async fn whep_endpoint(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    sfu_addr: web::Data<actix::Addr<crate::sfu::Sfu>>,
    path: web::Path<(String, String)>,
    session: web::ReqData<Session>,
    body: String,
) -> Result<HttpResponse, ApiError> {
    let (channel_uuid, stream_uuid) = path.into_inner();
    let user = session.principal.clone();

    // Content-Type checking
    let content_type = req.headers().get(header::CONTENT_TYPE);
    if content_type != Some(&header::HeaderValue::from_static("application/sdp")) {
        return Ok(
            HttpResponse::UnsupportedMediaType().body("Expected Content-Type: application/sdp")
        );
    }

    // Accept checking
    if let Some(accept) = req.headers().get(header::ACCEPT) {
        if accept != "application/sdp" {
            return Ok(HttpResponse::NotAcceptable().body("Expected Accept: application/sdp"));
        }
    }

    let answer = whep(
        &pool,
        channel_uuid,
        stream_uuid,
        user,
        sfu_addr.clone(),
        body,
    )
    .await?;

    Ok(HttpResponse::Created()
        .content_type("application/sdp")
        .body(answer))
}
