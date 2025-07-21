use crate::sfu::peer::Peer;
use crate::sfu::router::Router;

#[allow(dead_code)]
pub struct Lobby {
    id: String,
    peers: Vec<Peer>,
    router: Router,
}

impl Lobby {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            id: String::new(),
            peers: Vec::new(),
            router: Router::new(),
        }   
    }
}