use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use moq_relay::{Auth, Cluster};

/// Shared state passed to all web handler routes.
#[derive(Clone)]
pub struct RelayState {
    /// The authenticator for verifying incoming requests.
    pub auth: Auth,
    /// The cluster state for resolving origins.
    pub cluster: Cluster,
    /// TLS certificate information served at `/certificate.sha256`.
    pub tls_info: Arc<std::sync::RwLock<moq_native::ServerTlsInfo>>,
    /// Monotonically increasing connection counter for WebSocket sessions.
    pub conn_id: Arc<AtomicU64>,
}