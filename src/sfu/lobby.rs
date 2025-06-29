use crate::sfu::peer::Peer;
use crate::sfu::router::Router;

pub struct Lobby {
    id: String,
    peers: Vec<Peer>,
    router: Router,
}

impl Lobby {
    fn new() -> Self {
        Self {
            id: String::new(),
            peers: Vec::new(),
            router: Router::new(),
        }   
    }
}