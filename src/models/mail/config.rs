use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Clone)]
pub struct MailConfig {
    pub enable: bool,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub smtp_host: String,
    pub smtp_port: u16,
}
