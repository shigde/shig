use std::{collections::HashMap, net::SocketAddr};
use tokio::sync::mpsc::{self, Sender, Receiver};

pub struct Router {
    peers: HashMap<SocketAddr, Sender<(u32, Vec<u8>)>>, // ssrc, payload
}

impl Router {
    pub fn new() -> Self {
        Self { peers: HashMap::new() }
    }

    pub fn add_peer(&mut self, addr: SocketAddr, tx: Sender<(u32, Vec<u8>)>) {
        self.peers.insert(addr, tx);
    }

    pub async fn distribute(&self, from: SocketAddr, ssrc: u32, data: Vec<u8>) {
        for (addr, tx) in self.peers.iter() {
            if *addr != from {
                let _ = tx.send((ssrc, data.clone())).await;
            }
        }
    }
}
