use crate::sfu::lobby::Lobby;
use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::media::data_channel::{DataChannel, SdpMsgData};
use crate::sfu::media::error::{MediaError, MediaResult};
use crate::sfu::media::signaler::Signaler;
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
use webrtc::rtcp::payload_feedbacks::full_intra_request::{FirEntry, FullIntraRequest};
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::track::track_remote::TrackRemote;

#[derive(Clone)]
pub struct Receiver {
    pub id: PeerId,
    pc: Arc<RTCPeerConnection>,
    dc: Option<Arc<RTCDataChannel>>,
    #[allow(dead_code)]
    peer_addr: Addr<Peer>,
    lobby_addr: Addr<Lobby>,
    stop: CancellationToken,
    signaler: Signaler,
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
        let signaler = Signaler::new(id.clone(), peer_addr.clone());

        Ok(Self {
            id,
            pc,
            dc: None,
            peer_addr,
            lobby_addr,
            stop: CancellationToken::new(),
            signaler,
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
            let peer_stopped = self.stop.clone();
            pc.on_track(Box::new(
                move |track: Arc<TrackRemote>, _receiver, _streams| {
                    log::info!(
                            "receive (Receiver) remote-Track, kind={:?}, track_id={}, peer_id={}",
                            track.kind().clone(),
                            track.id().clone(),
                            peer_id.clone()
                        );

                    let cancel = CancellationToken::new();
                    let (rtp_tx, _dummy_rx) = broadcast::channel(32);
                    let media = Media::new(
                        peer_id.clone(),
                        track.id().clone(),
                        track.stream_id().clone(),
                        track.codec().capability.clone(),
                        track.kind().clone(),
                        rtp_tx.clone(),
                        cancel.clone(),
                    );

                    let media_id = media.id.clone();
                    lobby_addr.do_send(AddMedia { media });


                    let media_ssrc = track.ssrc();
                    let kind = track.kind().clone();
                    let mut seqno_fir = 0u8;
                    let pc_xx = pc_clone.clone();
                    // Send periodic PLI / FIR task
                    tokio::spawn(async move {
                        if kind == RTPCodecType::Video {
                            let mut result = Result::<usize, anyhow::Error>::Ok(0);
                            while result.is_ok() {
                                let timeout = tokio::time::sleep(Duration::from_secs(3));
                                tokio::pin!(timeout);

                                tokio::select! {
                            _ = timeout.as_mut() => {
                                result = pc_xx.write_rtcp(&[Box::new(FullIntraRequest {
                                    sender_ssrc: 0,
                                    media_ssrc,
                                    fir: vec![FirEntry {
                                        ssrc: media_ssrc,
                                        sequence_number: seqno_fir,
                                    }]
                                })]).await.map_err(Into::into);
                                seqno_fir = seqno_fir.wrapping_add(1);
                            }
                        }
                            }
                        }
                    });


                    let peer_id = peer_id.clone();
                    let peer_stopped = peer_stopped.clone();
                    let rtp_tx = rtp_tx.clone();
                    let lobby_addr = lobby_addr.clone();
                    // Spawn a background task that reads RTP packets (so we don't block the internal loop)
                    tokio::spawn(async move {
                        let kind = track.kind().clone();
                        log::info!(
                            "start reading (Receiver) remote-Track, kind={:?}, track_id={}, peer_id={}",
                            kind.clone(),
                            track.id().clone(),
                            peer_id.clone()
                        );
                        let _dummy_rx = rtp_tx.subscribe();

                        loop {
                            select! {
                                rtp_result = track.read_rtp() => {
                                    match rtp_result {
                                    Ok((rtp, _)) => {
                                        // Minimal: Packet size log (or simply ignore)
                                        // Warning: Frequent logs consume CPU; only sporadically useful here
                                        // println!("Got RTP payload len={}", rtp.payload.len());

                                            match rtp_tx.send(Arc::new(rtp)) {
                                                Ok(_) => {},
                                                Err(err) => {
                                                    log::error!("track (Receiver) send error: {err}, peer_id={}", peer_id.clone());
                                                }
                                            };
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
                                _ = peer_stopped.cancelled() => {
                                    log::info!("peer (Receiver) stopped all tracks, kind={:?}, track_id={}, peer_id={}",
                                        kind,
                                        track.id(),
                                        peer_id
                                    );
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

        // Add Transceiver for Reception
        // We expect the client to send video
        pc.add_transceiver_from_kind(RTPCodecType::Video, None)
            .await?;
        // We expect the client to send audio
        pc.add_transceiver_from_kind(RTPCodecType::Audio, None)
            .await?;

        let answer = self.create_answer(sdp_offer).await?;
        log::info!(
            "connecting (Receiver) and sending answer, peer_id={}",
            self.id
        );
        Ok(answer)
    }

    pub(crate) async fn on_signaling_offer(&mut self, offer_msg: SdpMsgData) -> MediaResult<()> {
        let answer = self.create_answer(offer_msg.sdp.as_str()).await?;
        log::info!("send (Receiver) signaling answer: peer_id={}", self.id);
        match self.signaler.send_answer(answer, offer_msg.number).await {
            Ok(_) => Ok(()),
            Err(e) => Err(MediaError::Renegotiation(format!("{:?}", e))),
        }
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
