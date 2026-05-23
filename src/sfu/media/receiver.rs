use crate::sfu::lobby::Lobby;
use crate::sfu::media::connector::{receiver_index, Connector, ConnectorType};
use crate::sfu::media::data_channel::{DataChannel, SdpMsgData};
use crate::sfu::media::error::{MediaError, MediaResult};
use crate::sfu::media::sdp::parse_offered_track_info;
use crate::sfu::media::track_info::InboundTrackInfo;
use crate::sfu::media::{AddMedia, Media, RemoveMedia};
use crate::sfu::peer::{Peer, PeerId};
use actix::Addr;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::track::track_remote::TrackRemote;

#[derive(Clone)]
pub struct Receiver {
    pub id: PeerId,
    pc: Arc<RTCPeerConnection>,
    // add the receiver dc to the sender signaler, because we're doing signaling over the receiver channel
    dc: Option<Arc<RTCDataChannel>>,
    #[allow(dead_code)]
    peer_addr: Addr<Peer>,
    lobby_addr: Addr<Lobby>,
    stop: CancellationToken,
    offered_track_infos: Vec<InboundTrackInfo>,
}

impl Connector for Receiver {
    fn get_pc(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.pc)
    }
}

impl DataChannel for Receiver {
    async fn set_dc(&mut self, dc: Arc<RTCDataChannel>) {
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
            stop: CancellationToken::new(),
            offered_track_infos: vec![],
        })
    }

    pub(crate) async fn connect(&mut self, sdp_offer: &str) -> MediaResult<String> {
        self.initialize_data_channel(self.peer_addr.clone(), ConnectorType::Receiver);
        let pc = Arc::clone(&self.pc);

        // 1) parse offer
        let offered_track_infos =
            parse_offered_track_info(sdp_offer).map_err(MediaError::SdpParse)?;

        // 2) Create Transceiver (BEFORE setRemoteDescription)
        // self.add_answerer_transceivers(&pc, &offered_track_infos)
        //     .await
        //     .map_err(MediaError::RTCCreate)?;

        self.offered_track_infos = offered_track_infos;

        // 3) on_track handler: read-only and discard (no decoding/rendering)
        {
            let pc_clone = Arc::clone(&self.pc);
            let peer_id = self.id.clone();
            let lobby_addr = self.lobby_addr.clone();
            let peer_stopped = self.stop.clone();
            let offered_track_infos = self.offered_track_infos.clone();
            pc.on_track(Box::new(
                move |track: Arc<TrackRemote>, receiver, _streams| {
                    let pc = Arc::clone(&pc_clone);
                    let peer_id = peer_id.clone();
                    let lobby_addr = lobby_addr.clone();
                    let peer_stopped = peer_stopped.clone();
                    let offered_track_infos = offered_track_infos.clone();

                    log::info!(
                        "Received TRACK kind={:?} ssrc={} pt={} codec={} fmtp={}",
                        track.kind(),
                        track.ssrc(),
                        track.payload_type(),
                        track.codec().capability.mime_type,
                        track.codec().capability.sdp_fmtp_line,
                    );

                    Box::pin(async move {
                        let Ok(index) = receiver_index(Arc::clone(&pc), &receiver).await else {
                            log::warn!("on track failed to find receiver, peer_id={}", peer_id);
                            return;
                        };
                        let mid = offered_track_infos[index].mid.clone();
                        let purpose = offered_track_infos[index].purpose.clone();
                        let info = offered_track_infos[index].info.clone();
                        let is_muted = offered_track_infos[index].muted;
                        log::info!(
                            "receive (Receiver) remote-Track, kind={:?}, track_id={}, mid={:?}, peer_id={}",
                            track.kind(),
                            track.id(),
                            mid,
                            peer_id
                        );

                        let cancel = CancellationToken::new();
                        let (rtp_tx, _dummy_rx) = broadcast::channel(2048);

                        let media = Media::new(
                            peer_id.clone(),
                            mid,
                            track.id().clone(),
                            track.stream_id().clone(),
                            track.codec().capability.clone(),
                            track.kind(),
                            rtp_tx.clone(),
                            cancel.clone(),
                            is_muted,
                            purpose,
                            info,
                            track.payload_type().clone()
                        );

                        let media_id = media.id.clone();
                        lobby_addr.do_send(AddMedia { media });

                        let media_ssrc = track.ssrc();
                        let kind = track.kind();
                        // let mut seqno_fir = 0u8;
                        let pc_xx = Arc::clone(&pc);

                        // Send periodic PLI / FIR
                        tokio::spawn(async move {
                            if kind != RTPCodecType::Video {
                                return;
                            }

                            loop {
                                if let Err(err) = pc_xx
                                    .write_rtcp(&[Box::new(PictureLossIndication {
                                        sender_ssrc: 0,
                                        media_ssrc,
                                    })])
                                    .await
                                {
                                    log::warn!("send PLI failed: {err}");
                                    break;
                                }

                                tokio::time::sleep(Duration::from_millis(300)).await;
                            }
                        });

                        // RTP reader task
                        let rtp_tx = rtp_tx.clone();
                        let lobby_addr = lobby_addr.clone();

                        tokio::spawn(async move {
                            log::info!(
                                "start reading (Receiver) remote-Track, kind={:?}, track_id={}, peer_id={}",
                                kind,
                                track.id(),
                                peer_id
                            );

                            let _dummy_rx = rtp_tx.subscribe();

                            loop {
                                select! {
                                    rtp_result = track.read_rtp() => {
                                        match rtp_result {
                                            Ok((rtp, _)) => {
                                                let _ = rtp_tx.send(Arc::new(rtp));
                                            }
                                            Err(err) => {
                                                log::error!(
                                                    "track (Receiver) read error: {err}, peer_id={}",
                                                    peer_id
                                                );
                                                lobby_addr.do_send(RemoveMedia { media_id });
                                                cancel.cancel();
                                                break;
                                            }
                                        }
                                    }
                                    _ = peer_stopped.cancelled() => {
                                        log::info!(
                                            "peer (Receiver) stopped track, kind={:?}, track_id={}, peer_id={}",
                                            kind,
                                            track.id(),
                                            peer_id
                                        );
                                        cancel.cancel();
                                        break;
                                    }
                                }
                            }
                        });
                    })
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

    pub(crate) async fn on_signaling_offer(
        &mut self,
        offer_msg: SdpMsgData,
    ) -> MediaResult<String> {
        self.create_answer(offer_msg.sdp.as_str())
            .await
            .map(|answer| {
                log::info!(
                    "create (Receiver) signaling answer success: peer_id={}",
                    self.id
                );
                answer
            })
            .map_err(|err| {
                log::error!(
                    "create (Receiver) signaling answer failed: peer_id={}, error={}",
                    self.id,
                    err
                );
                err
            })
    }

    pub(crate) async fn shutdown(&self) {
        log::info!("shutdown (Receiver), peer_id={}", self.id);

        self.stop.cancel();

        if let Some(dc) = self.get_dc() {
            if let Err(e) = dc.close().await {
                log::error!(
                    "close data channel (Receiver) error: {e}, peer_id={}",
                    self.id
                );
            }
        }

        let pc = self.get_pc();
        if let Err(e) = pc.close().await {
            log::error!(
                "close peer_connection (Receiver) error: {e}, peer_id={}",
                self.id
            );
        }
    }
}
