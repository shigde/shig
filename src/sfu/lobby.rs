use std::collections::HashMap;
use crate::sfu::peer::Peer;
use crate::sfu::router::Router;
use actix::{Actor, Context};

#[allow(dead_code)]
pub struct Lobby {
    id: String,
    peers: HashMap<String, Peer>,
    router: Router,
}

impl Lobby {
    fn new(id: String) -> Self {
        Self {
            id,
            peers: HashMap::new(),
            router: Router::new(),
        }
    }
}

impl Actor for Lobby {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        log::info!("started: lobby actor {} is alive", self.id);
    }

    fn stopped(&mut self, ctx: &mut Context<Self>) {
        log::info!("stopped: lobby actor {} is stopped", self.id);
    }
}
