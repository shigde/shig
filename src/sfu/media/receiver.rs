use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::media::error::MediaResult;
use crate::sfu::peer::Peer;
use actix::Addr;
use std::sync::Arc;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::track::track_remote::TrackRemote;

pub struct Receiver {
    id: String,
    pc: Arc<RTCPeerConnection>,
    peer_addr: Addr<Peer>,
}

impl Connector for Receiver {
    fn get_pc(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.pc)
    }
}

impl Receiver {
    pub(crate) async fn new(id: String, peer_addr: Addr<Peer>) -> MediaResult<Self> {
        let pc =
            Self::create_connection(id.clone(), peer_addr.clone(), ConnectorType::Receiver).await?;
        Ok(Self { id, pc, peer_addr })
    }

    pub(crate) async fn connect(&self, sdp_offer: &str) -> MediaResult<String> {
        let pc = Arc::clone(&self.pc);
        // 2) on_track handler: read-only and discard (no decoding/rendering)
        {
            let pc_clone = Arc::clone(&self.pc);
            let id = self.id.clone();
            pc.on_track(Box::new(
                move |track: Arc<TrackRemote>, _receiver, _streams| {
                    let t = Arc::clone(&track);
                    let id_clone = id.clone();
                    // Spawn a background task that reads RTP packets (so we don't block the internal loop)
                    tokio::spawn(async move {
                        let kind = t.kind();
                        log::info!(
                            "New Remote-Track: kind={:?}, track_id={}, peer_id={}",
                            kind,
                            t.id(),
                            id_clone
                        );
                        // Option: only handle video
                        if kind == RTPCodecType::Video {
                            loop {
                                match t.read_rtp().await {
                                    Ok((rtp, _)) => {
                                        // Minimal: Packet size log (or simply ignore)
                                        // Warning: Frequent logs consume CPU; only sporadically useful here
                                        // println!("Got RTP payload len={}", rtp.payload.len());
                                        let _ = rtp; // wir verwerfen den Inhalt
                                    }
                                    Err(err) => {
                                        log::error!(
                                            "Track read error (closing): {err}, peer_id={}",
                                            id_clone
                                        );
                                        break;
                                    }
                                }
                            }
                        } else {
                            // If audio is received: reject it as well.
                            loop {
                                if t.read_rtp().await.is_err() {
                                    break;
                                }
                            }
                        }
                        // When the track ends, automatically clean up.
                        let _ = pc_clone; // If you want to do some cleanup at the end
                    });
                    Box::pin(async {})
                },
            ));
        }

        let answer = self.create_answer(sdp_offer).await?;
        Ok(answer)
    }
}
