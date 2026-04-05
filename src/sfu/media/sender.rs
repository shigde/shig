use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::media::data_channel::{DataChannel, SdpMsgData};
use crate::sfu::media::error::{MediaError, MediaResult};
use crate::sfu::media::sdp::set_track_info;
use crate::sfu::media::track_info::OutboundTrackInfo;
use crate::sfu::media::{Media, MediaId};
use crate::sfu::peer::{Peer, PeerId};
use actix::Addr;
use std::collections::HashMap;
use std::sync::Arc;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;
use webrtc::rtp_transceiver::RTCRtpTransceiverInit;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocal;

#[derive(Clone)]
pub struct Sender {
    id: PeerId,
    pc: Arc<RTCPeerConnection>,
    // We are ignoring this DataChannel for now @see Handler<OnDataChannel> for Peer,
    // every message exchange is sent via the Receiver DataChannel
    dc: Option<Arc<RTCDataChannel>>,
    #[allow(dead_code)]
    peer_addr: Addr<Peer>,
    // MediaId -> RTCRtpTransceiver
    // This is used to send remote mute messages to the peer and identify the track by the mid
    sending_media: HashMap<MediaId, OutboundTrackInfo>,
}

impl Connector for Sender {
    fn get_pc(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.pc)
    }
}

impl DataChannel for Sender {
    async fn set_dc(&mut self, dc: Arc<RTCDataChannel>) {
        self.dc = Some(dc);
    }

    fn get_dc(&self) -> Option<Arc<RTCDataChannel>> {
        self.dc.clone()
    }
}

impl Sender {
    pub(crate) async fn new(id: PeerId, peer_addr: Addr<Peer>) -> MediaResult<Self> {
        let pc =
            Self::create_connection(id.clone(), peer_addr.clone(), ConnectorType::Sender).await?;
        // let signaler = Signaler::new(id.clone(), peer_addr.clone());
        Ok(Self {
            id,
            pc,
            dc: None,
            peer_addr,
            sending_media: HashMap::new(),
        })
    }

    pub(crate) async fn setup_offer(&mut self) -> MediaResult<String> {
        if let Err(e) = self
            .create_data_channel(self.peer_addr.clone(), ConnectorType::Sender)
            .await
        {
            return Err(MediaError::DataCannel(e.to_string()));
        }
        log::info!("connect and create answer (Sender), peer_id={}", self.id,);

        let offer = self.create_offer().await?;
        let vec: Vec<&OutboundTrackInfo> = self.sending_media.values().collect();
        let parsed_offer = set_track_info(offer, vec)
            .map_err(|e| MediaError::SdpParse(anyhow::anyhow!("set_track_info failed: {}", e)))?;

        Ok(parsed_offer.sdp.to_string())
    }

    pub async fn add_media(&mut self, media: Media) -> MediaResult<()> {
        log::info!("add track (Sender), peer_id={}", self.id);

        let track = Arc::new(TrackLocalStaticRTP::new(
            media.capability.clone(),
            media.id.to_string(),
            media.src_stream_id.clone(),
        ));

        let pc = self.get_pc();

        let transceiver = pc
            .add_transceiver_from_track(
                Arc::clone(&track) as Arc<dyn TrackLocal + Send + Sync>,
                Some(RTCRtpTransceiverInit {
                    direction: RTCRtpTransceiverDirection::Sendonly,
                    send_encodings: vec![],
                }),
            )
            .await?;

        log::info!("track added (Sender), peer_id={}", self.id);
        let msid = format!("{} {}", media.src_stream_id.clone(), media.id.clone());
        let track_info = OutboundTrackInfo::new(
            msid,
            transceiver,
            media.purpose.clone(),
            media.muted,
            media.info.clone(),
        );
        self.sending_media
            .insert(media.id.clone(), track_info.clone());
        media.subscribe(track).await;
        Ok(())
    }


    pub async fn remove_track(&mut self, media_id: MediaId) -> MediaResult<()> {
        let media_id_string = media_id.to_string();
        log::info!("remove track (Sender) peer_id={}", self.id);

        let senders = self.pc.get_senders().await;

        for sender in senders {
            if let Some(track) = sender.track().await {
                if track.id() == media_id_string {
                    self.pc.remove_track(&sender).await?;
                    self.sending_media.remove(&media_id);
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    // pub async fn remove_track(&mut self, media_id: MediaId) -> MediaResult<()> {
    //     let media_id_string = media_id.to_string();
    //     log::info!("remove track (Sender) peer_id={}", self.id,);
    //     for sender in self.pc.get_senders().await.iter() {
    //         if let Some(sender_track) = sender.track().await {
    //             if sender_track.id() == media_id_string {
    //                 if let Err(e) = self.pc.remove_track(sender).await {
    //                     return Err(e.into());
    //                 }
    //                 self.sending_media.remove(&media_id);
    //             }
    //         }
    //     }
    //     Ok(())
    // }

    pub async fn create_signal_offer(&mut self) -> MediaResult<String> {
        log::info!("create (Sender) signaling offer start, peer_id={}", self.id);
        let pc = self.get_pc();
        let offer = match pc.create_offer(None).await {
            Ok(o) => o,
            Err(e) => {
                log::error!("create_offer failed, peer_id={}, error={}", self.id, e);
                return Err(MediaError::Renegotiation(e.to_string()));
            }
        };

        if let Err(e) = pc.set_local_description(offer.clone()).await {
            log::error!(
                "set_local_description failed, peer_id={}, error={}",
                self.id,
                e
            );
            return Err(MediaError::Renegotiation(e.to_string()));
        }

        log::info!(
            "create (Sender) signaling offer success, peer_id={}",
            self.id
        );

        let vec: Vec<&OutboundTrackInfo> = self.sending_media.values().collect();
        let parsed_offer = set_track_info(offer, vec)
            .map_err(|e| MediaError::SdpParse(anyhow::anyhow!("set_track_info failed: {}", e)))?;

        Ok(parsed_offer.sdp.to_string())
    }

    pub async fn set_signal_answer(&mut self, msg: SdpMsgData) -> MediaResult<()> {
        log::info!("receive (Sender) signaling answer, peer_id={}", self.id);
        self.set_answer(msg.sdp.as_str()).await
    }

    pub fn get_mid_and_mute(&mut self, media_id: MediaId, muted: bool) -> Option<String> {
        log::info!("send mute remote (Sender), peer_id={}", self.id);
        let Some(info) = self.sending_media.get_mut(&media_id) else {
            log::warn!("mute remote track not found, peer_id={}", self.id);
            return None;
        };
        info.muted = muted;
        let Some(mid) = info.transceiver.mid() else {
            return None;
        };
        Some(mid.to_string())
    }

    pub(crate) async fn shutdown(&self) {
        log::info!("shutdown (Sender), peer_id={}", self.id);

        if let Some(dc) = self.get_dc() {
            if let Err(e) = dc.close().await {
                log::error!(
                    "close data channel (Sender) error: {e}, peer_id={}",
                    self.id
                );
            }
        }

        let pc = self.get_pc();
        if let Err(e) = pc.close().await {
            log::error!(
                "close peer_connection (Sender) error: {e}, peer_id={}",
                self.id
            );
        }
    }
}
