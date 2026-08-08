use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct MailConfig {
    #[allow(dead_code)]
    pub enable: bool,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub smtp_host: String,
    pub smtp_port: u16,
}
