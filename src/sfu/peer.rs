use futures::StreamExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{net::SocketAddr, sync::Arc, time::Instant};
use str0m::bwe::Bitrate;
use str0m::{media::MediaKind, net::Receive, Rtc, RtcConfig};
use tokio::{net::UdpSocket, sync::mpsc::Sender, time::sleep};

pub struct Peer {
    pub id: u64,
    pub addr: SocketAddr,
    pub rtc: Rtc,
    udp_socket: Option<UdpSocket>,
    socket_addr: Option<SocketAddr>,
}

impl Peer {
    pub fn new(addr: SocketAddr) -> Self {
        static ID_COUNTER: AtomicU64 = AtomicU64::new(0);
        let next_id = ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        let rtc = RtcConfig::default()
            .enable_bwe(Some(Bitrate::kbps(1024)))
            .clear_codecs()
            .enable_opus(true)
            .enable_vp8(true)
            .enable_vp9(true)
            .enable_h264(true)
            //.set_rtp_mode(true)
            .build();

        Self {
            id: next_id,
            addr,
            rtc,
            udp_socket: None,
            socket_addr: None,
        }
    }

    pub async fn run(
        self,
        socket: Arc<UdpSocket>,
        // media_tx: Sender<(SocketAddr, u32, Vec<u8>)>, // (von, SSRC, Payload)
    ) {
        let addr = self.addr;
        // let mut rtc = self.rtc;

        let socket_recv = socket.clone();
        // let socket_send = socket.clone();

        // Empfänger
        let recv_task = tokio::spawn(async move {
            let mut buf = [0u8; 1500];
            loop {
                let (len, from) = socket_recv.recv_from(&mut buf).await.unwrap();
                if from != addr {
                    continue;
                }
                let packet = &buf[..len];
                //rtc.handle_input(Instant::now(), Receive::from_bytes(packet.to_vec(), from)).unwrap();

                // Media empfangen?
                // while let Some(event) = rtc.poll_output().next() {
                //     if let str0m::RtcEvent::MediaData(media) = event {
                //         let _ = media_tx.send((addr, media.ssrc(), media.data)).await;
                //     }
                // }
            }
        });

        // Sender
        let send_task = tokio::spawn(async move {
            loop {
                //while let Some(out) = rtc.poll_output().next() {
                // if let str0m::RtcOutput::Transmit(tx) = out {
                //     let _ = socket_send.send_to(&tx.payload, tx.dst_addr).await;
                // }
                //}
                sleep(std::time::Duration::from_millis(10)).await;
            }
        });

        let _ = tokio::join!(recv_task, send_task);
    }
}
