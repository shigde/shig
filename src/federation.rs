use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct FederationConfig {
    enable: bool,
    domain: String,
    instance: String,
    token: String,
    tls: bool,
}
