use actix_web::{error::ErrorInternalServerError, http::StatusCode, HttpResponse};
use axum::body::{to_bytes, Body};
use axum::http::Request;
use tower::ServiceExt;

/// Shared relay handles; upstream owns authentication and certificate rotation.
#[derive(Clone)]
pub struct RelayState {
    pub auth: moq_relay::auth::Auth,
    pub cluster: moq_relay::cluster::Cluster,
    #[allow(dead_code)]
    pub web: axum::Router,
}

impl RelayState {
    // this method is used to forward http requests to the web server in fingerprint api
    #[allow(dead_code)]
    pub async fn forward_http(&self, uri: String) -> actix_web::Result<HttpResponse> {
        let request = Request::builder()
            .uri(uri)
            .body(Body::empty())
            .map_err(ErrorInternalServerError)?;
        let response = self
            .web
            .clone()
            .oneshot(request)
            .await
            .map_err(ErrorInternalServerError)?;
        let status =
            StatusCode::from_u16(response.status().as_u16()).map_err(ErrorInternalServerError)?;
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .cloned();
        let body = to_bytes(response.into_body(), 8 * 1024 * 1024)
            .await
            .map_err(ErrorInternalServerError)?;
        let mut response = HttpResponse::build(status);
        if let Some(content_type) = content_type {
            response.insert_header((
                "content-type",
                content_type.to_str().map_err(ErrorInternalServerError)?,
            ));
        }
        Ok(response.body(body))
    }
}
