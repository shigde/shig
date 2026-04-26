#[derive(Debug, serde::Deserialize)]
pub(crate) struct AuthQuery {
    pub(crate) jwt: Option<String>,
    pub(crate) register: Option<String>,
}