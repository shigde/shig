use std::net::SocketAddr;
use std::sync::Arc;
use derive_more::Display;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;
use webrtc::rtp::packet::Packet;
use webrtc::util::Marshal;
use crate::sfu::relay::error::{RelayError, RelayResult};

#[derive(Debug, Display, PartialEq, Clone, Copy)]
pub enum RtpForwarderKind {
    #[display(fmt = "Video-Forwarder")]
    Video,
    #[display(fmt = "Audio-Forwarder")]
    Audio,
}

pub async fn forward_rtp_sender_to_udp(
    mut rx: broadcast::Receiver<Arc<Packet>>,
    target: SocketAddr,
    cancel: CancellationToken,
    mut ffmpeg_ready_rx: watch::Receiver<bool>,
    kind: RtpForwarderKind,
) -> RelayResult<()> {
    let socket = UdpSocket::bind("127.0.0.1:0").await.map_err(|e| {
        RelayError::RtpForwarder(format!("Could not bind to UDP socket: {}", e.to_string()))
    })?;

    let mut seen_sps = false;
    let mut seen_pps = false;
    let mut ready = match kind {
        RtpForwarderKind::Video => false,
        RtpForwarderKind::Audio => true,
    };

    // wait for ffmpeg publisher ready
    while !*ffmpeg_ready_rx.borrow() {
        select! {
                _ = cancel.cancelled() => {
                    log::info!("RTP forwarder {}, cancelled while waiting for ffmpeg ready", kind);
                    return Ok(());
                }

                changed = ffmpeg_ready_rx.changed() => {
                    if changed.is_err() {
                        return Err(RelayError::RtpForwarder(
                            format!("RTP forwarder {},ready channel closed", kind)
                        ));
                    }
                }
            }
    }

    log::info!("RTP forwarder {}, start forwarding rtp pkgs to udp", kind);
    loop {
        select! {
            _ = cancel.cancelled() => {
                log::info!("RTP forwarder {} cancelled: {}", kind, target);
                return Ok(());
            }

            result = rx.recv() => {
                match result {
                    Ok(packet) => {
                        if packet.payload.is_empty() {
                            continue;
                        }
                        if !ready && kind == RtpForwarderKind::Video {
                            let info = inspect_h264_rtp_payload(&packet.payload);

                            if info.sps {
                                seen_sps = true;
                            }

                            if info.pps {
                                seen_pps = true;
                            }
                            // Forward SPS/PPS anyway!
                            if info.sps || info.pps {
                                let raw = packet.marshal().map_err(|e| {
                                    RelayError::RtpForwarder(format!("RTP forwarder {}, marshal rtp pkg: {}", kind, e.to_string()))
                                })?;
                                socket.send_to(&raw, target).await.map_err(|e| {
                                    RelayError::RtpForwarder(format!("RTP forwarder {}, sending rtp pkg: {}", kind, e.to_string()))
                                })?;
                                continue;
                            }

                            if seen_sps && seen_pps && info.idr && !ready{
                                ready = true;
                                log::info!("RTP forwarder {}, codec for H264 ready: SPS + PPS + IDR", kind);
                            } else {
                                continue;
                            }
                        }

                        let raw = packet.marshal().map_err(|e| {
                            RelayError::RtpForwarder(format!("Marshal rtp pkg: {}", e.to_string()))
                        })?;
                        socket.send_to(&raw, target).await.map_err(|e| {
                            RelayError::RtpForwarder(format!("Sending rtp pkg: {}", e.to_string()))
                        })?;
                    }

                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        log::warn!("RTP forwarder {} lagged, skipped {} packets for {}", kind, skipped, target);
                        continue;
                    }

                    Err(broadcast::error::RecvError::Closed) => {
                        log::info!("RTP forwarder {} closed for {}", kind, target);
                        return Ok(());
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct H264Info {
    sps: bool,
    pps: bool,
    idr: bool,
}

fn inspect_h264_rtp_payload(payload: &[u8]) -> H264Info {
    let mut info = H264Info {
        sps: false,
        pps: false,
        idr: false,
    };

    if payload.is_empty() {
        return info;
    }

    let nal_type = payload[0] & 0x1F;

    match nal_type {
        5 => info.idr = true,
        7 => info.sps = true,
        8 => info.pps = true,

        // STAP-A: mehrere NALUs in einem RTP packet
        24 => {
            let mut i = 1;
            while i + 2 <= payload.len() {
                let size = u16::from_be_bytes([payload[i], payload[i + 1]]) as usize;
                i += 2;

                if i + size > payload.len() || size == 0 {
                    break;
                }

                let inner_type = payload[i] & 0x1F;

                match inner_type {
                    5 => info.idr = true,
                    7 => info.sps = true,
                    8 => info.pps = true,
                    _ => {}
                }

                i += size;
            }
        }

        // FU-A: fragmentierte NALU
        28 if payload.len() >= 2 => {
            let fu_header = payload[1];
            let start = fu_header & 0x80 != 0;
            let original_type = fu_header & 0x1F;

            if start {
                match original_type {
                    5 => info.idr = true,
                    7 => info.sps = true,
                    8 => info.pps = true,
                    _ => {}
                }
            }
        }

        _ => {}
    }

    info
}
