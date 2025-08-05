use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::sfu::Sfu;
use actix::Addr;
use actix_web::web;
use serde::{Deserialize, Serialize};
use crate::models::error::ApiError;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Lobby {
    pub uuid: String,
    // user name@domain
    pub user: String,
    pub name: String,
    pub description: String,
    pub support: String,
    pub is_open: bool,
    pub banner_name: String,
}

impl Lobby {
    pub fn new() -> Self {
        Self {
            uuid: String::new(),
            user: String::new(),
            name: String::new(),
            description: String::new(),
            support: String::new(),
            is_open: false,
            banner_name: String::new(),
        }
    }

    pub(crate) fn open(
        pool: &web::Data<DbPool>,
        stream_id: String,
        user: Principal,
        sfu_addr: web::Data<Addr<Sfu>>,
    ) -> Result<Self, ApiError> {
        let mut lobby = Lobby::new();
        lobby.is_open = true;
        Ok(lobby)
    }
}
