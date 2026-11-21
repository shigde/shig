use actix_web::{error::ErrorInternalServerError, http::StatusCode, HttpResponse};
use axum::body::{to_bytes, Body};
use axum::http::Request;
use tower::ServiceExt;

/// Shared relay handles; upstream owns authentication and certificate rotation.
#[derive(Clone)]
pub struct RelayState {
    pub h264_diagnostics_directory: Option<std::path::PathBuf>,
    pub media: crate::sfu::media::MediaService,
    /// Clone of the relay's shared MoQ traffic registry for admin monitoring.
    pub relay_stats: moq_net::stats::Registry,
    /// In-process router for the relay's full Prometheus exposition endpoint.
    pub relay_metrics: axum::Router,
    pub auth: moq_relay::auth::Auth,
    pub cluster: moq_relay::cluster::Cluster,
    #[allow(dead_code)]
    pub web: axum::Router,
}

impl RelayState {
    /// Render the upstream relay's own Prometheus endpoint in-process.
    pub async fn prometheus_metrics(&self) -> actix_web::Result<String> {
        let request = Request::builder()
            .uri("/metrics")
            .body(Body::empty())
            .map_err(ErrorInternalServerError)?;
        let response = self
            .relay_metrics
            .clone()
            .oneshot(request)
            .await
            .map_err(ErrorInternalServerError)?;
        if !response.status().is_success() {
            return Err(ErrorInternalServerError(format!(
                "Relay metrics route returned {}",
                response.status()
            )));
        }
        let body = to_bytes(response.into_body(), 8 * 1024 * 1024)
            .await
            .map_err(ErrorInternalServerError)?;
        String::from_utf8(body.to_vec()).map_err(ErrorInternalServerError)
    }

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
