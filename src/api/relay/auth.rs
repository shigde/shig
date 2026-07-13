#[derive(Debug, serde::Deserialize)]
pub(crate) struct AuthQuery {
    pub(crate) jwt: Option<String>,
    #[allow(dead_code)]
    pub(crate) register: Option<String>,
}