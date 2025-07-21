use std::collections::HashMap;
use crate::sfu::config::SfuConfig;
use crate::sfu::lobby::Lobby;

pub mod config;
pub mod peer;
pub mod router;
pub mod error;
pub mod lobby;

#[allow(dead_code)]
struct Sfu {
    config: SfuConfig,
    // No need for Arc amd Mutex here, because the Conferences List is not shared
    lobbies: Box<HashMap<String, Lobby>>,
}

impl Sfu {
    #[allow(dead_code)]
    pub fn new(config: SfuConfig) -> Sfu {
        let lobbies = Box::new(HashMap::new());
        Sfu {
            config,
            lobbies,
        }
    }
}

#[allow(dead_code)]
pub fn run () {
    //let socket = Arc::new(UdpSocket::bind("0.0.0.0:5000").await.unwrap());
    // let mut sfu = Router::new();
    // let mut seen = HashMap::new();
    // 

    loop {
        // let (len, addr) = socket_clone.recv_from(&mut buf).await.unwrap();
        // if seen.contains_key(&addr) {
        //     continue;
        // }
        // 
        // println!("[+] Neuer Peer: {addr}");
        // 
        // let (media_tx, mut media_rx) = mpsc::channel(64);
        // let (forward_tx, mut forward_rx) = mpsc::channel(64);
        // 
        // // In Router eintragen
        // sfu.add_peer(addr, forward_tx.clone());
        // 
        // // Peer starten
        // let socket_peer = socket.clone();
        // tokio::spawn(async move {
        //     let peer = peer::Peer::new(addr);
        //     peer.run(socket_peer, media_tx).await;
        // });
        // 
        // // Weiterleitung: Media → andere Peers
        // let sfu_clone = sfu.clone();
        // tokio::spawn(async move {
        //     while let Some((from, ssrc, data)) = media_rx.recv().await {
        //         sfu_clone.distribute(from, ssrc, data).await;
        //     }
        // });
        // 
        // // Eingehende Pakete für diesen Peer senden
        // let socket_send = socket.clone();
        // tokio::spawn(async move {
        //     while let Some((ssrc, payload)) = forward_rx.recv().await {
        //         let _ = socket_send.send_to(&payload, addr).await;
        //     }
        // });
        // 
        // seen.insert(addr, true);
    }
}