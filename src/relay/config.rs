use clap::Parser;
use moq_relay::{AuthConfig, ClusterConfig, WebConfig};
use serde::{Deserialize, Serialize};

#[derive(Parser, Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields, default)]
#[non_exhaustive]
pub struct RelayConfig {

    /// The QUIC/TLS configuration for the server.
    #[command(flatten)]
    #[serde(default)]
    pub server: moq_native::ServerConfig,

    /// The QUIC/TLS configuration for the client. (clustering only)
    #[command(flatten)]
    #[serde(default)]
    pub client: moq_native::ClientConfig,

    /// Log configuration.
    #[command(flatten)]
    #[serde(default)]
    pub log: moq_native::Log,

    /// Cluster configuration.
    #[command(flatten)]
    #[serde(default)]
    pub cluster: ClusterConfig,

    /// Authentication configuration.
    #[command(flatten)]
    #[serde(default)]
    pub auth: AuthConfig,

    /// Optionally run a TCP HTTP/WebSocket server.
    #[command(flatten)]
    #[serde(default)]
    pub web: WebConfig,
    /// If provided, load the configuration from this file.
    #[serde(default)]
    pub file: Option<String>,

    /// Iroh specific configuration, used for both a client and server.
    #[command(flatten)]
    #[serde(default)]
    pub iroh: moq_native::IrohEndpointConfig,
}