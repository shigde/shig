use crate::sfu::lobby::Lobby;
use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::media::data_channel::{DataChannel, DataChannelMsg, SdpMsgData};
use crate::sfu::media::error::{MediaError, MediaResult};
use crate::sfu::media::{AddMedia, Media, RemoveMedia};
use crate::sfu::peer::{Peer, PeerId};
use actix::Addr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_remote::TrackRemote;

#[derive(Clone)]
pub struct Receiver {
    pub id: PeerId,
    pc: Arc<RTCPeerConnection>,
    dc: Option<Arc<RTCDataChannel>>,
    #[allow(dead_code)]
    peer_addr: Addr<Peer>,
    lobby_addr: Addr<Lobby>,
}

impl Connector for Receiver {
    fn get_pc(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.pc)
    }
}

impl DataChannel for Receiver {
    fn set_dc(&mut self, dc: Arc<RTCDataChannel>) {
        self.dc = Some(dc);
    }

    fn get_dc(&self) -> Option<Arc<RTCDataChannel>> {
        self.dc.clone()
    }
}

impl Receiver {
    pub(crate) async fn new(
        id: PeerId,
        peer_addr: Addr<Peer>,
        lobby_addr: Addr<Lobby>,
    ) -> MediaResult<Self> {
        let pc =
            Self::create_connection(id.clone(), peer_addr.clone(), ConnectorType::Receiver).await?;
        Ok(Self {
            id,
            pc,
            dc: None,
            peer_addr,
            lobby_addr,
        })
    }

    pub(crate) async fn connect(&mut self, sdp_offer: &str) -> MediaResult<String> {
        self.initialize_data_channel(self.peer_addr.clone(), ConnectorType::Receiver);
        let pc = Arc::clone(&self.pc);
        // 2) on_track handler: read-only and discard (no decoding/rendering)
        {
            let pc_clone = Arc::clone(&self.pc);
            let peer_id = self.id.clone();
            let lobby_addr = self.lobby_addr.clone();
            pc.on_track(Box::new(
                move |track: Arc<TrackRemote>, _receiver, _streams| {
                    let cancel = CancellationToken::new();
                    let (rtp_tx, _) = broadcast::channel(32);
                    let media = Media::new(
                        peer_id.clone(),
                        track.id().clone(),
                        track.stream_id().clone(),
                        track.kind().clone(),
                        track.codec().capability.mime_type.clone(),
                        rtp_tx.clone(),
                        cancel.clone(),
                    );

                    let media_id = media.id.clone();
                    lobby_addr.do_send(AddMedia { media });

                    let peer_id = peer_id.clone();
                    let rtp_tx = rtp_tx.clone();
                    let lobby_addr = lobby_addr.clone();
                    // Spawn a background task that reads RTP packets (so we don't block the internal loop)
                    tokio::spawn(async move {
                        let kind = track.kind();
                        log::info!(
                            "receive (Receiver) new Remote-Track, kind={:?}, track_id={}, peer_id={}",
                            kind,
                            track.id(),
                            peer_id
                        );

                        loop {
                            match track.read_rtp().await {
                                Ok((rtp, _)) => {
                                    // Minimal: Packet size log (or simply ignore)
                                    // Warning: Frequent logs consume CPU; only sporadically useful here
                                    // println!("Got RTP payload len={}", rtp.payload.len());
                                    let _ = rtp_tx.send(Arc::new(rtp));
                                }
                                Err(err) => {
                                    log::error!(
                                        "track (Receiver) read error (closing): {err}, peer_id={}",
                                        peer_id.clone()
                                    );
                                    lobby_addr.do_send(RemoveMedia { media_id });
                                    cancel.cancel();
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
        log::info!(
            "connecting (Receiver) and sending answer, peer_id={}",
            self.id
        );
        Ok(answer)
    }

    pub(crate) async fn on_signaling_offer(&mut self, offer_msg: SdpMsgData) -> MediaResult<()> {
        let answer = self.create_answer(offer_msg.sdp.as_str()).await?;
        let answer_msg = DataChannelMsg::AnswerMsg(SdpMsgData {
            number: offer_msg.number,
            sdp: answer,
        });

        log::info!("send (Receiver) signaling answer: peer_id={}", self.id);
        match self.send_dcm(answer_msg).await {
            Ok(_) => Ok(()),
            Err(e) => Err(MediaError::Renegotiation(format!("{:?}", e))),
        }
    }
}
