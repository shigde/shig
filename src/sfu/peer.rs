use actix::{Actor, Context};

pub struct Peer {
    pub id: String,
}

impl Peer {
    fn new(id: String) -> Self {
        Self { id }
    }
}

impl Actor for Peer {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        log::info!("started: peer actor {} is alive", self.id);
    }

    fn stopped(&mut self, ctx: &mut Context<Self>) {
        log::info!("stopped: peer actor {} is stopped", self.id);
    }
}
