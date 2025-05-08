use crate::federation::FederationConfig;
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub domain: String,
    pub tls: bool,
    pub instance: String,
}

impl Settings {
    pub fn new(cfg: &web::Data<FederationConfig>) -> Settings {
        Settings {
            domain: cfg.domain.clone(),
            tls: cfg.tls.clone(),
            instance: cfg.instance.clone(),
        }
    }
}
