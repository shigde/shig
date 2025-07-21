use std::{net::SocketAddr};
use tokio::{net::UdpSocket};

#[allow(dead_code)]
pub struct Peer {
    pub id: u64,
    pub addr: SocketAddr,
    udp_socket: Option<UdpSocket>,
    socket_addr: Option<SocketAddr>,
}

