use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::media::data_channel::{DataChannel, DataChannelMsg, SdpMsgData};
use crate::sfu::media::error::{MediaError, MediaResult};
use crate::sfu::media::Media;
use crate::sfu::peer::{Peer, PeerId};
use actix::Addr;
use std::sync::Arc;
use uuid::Uuid;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocal;

#[derive(Clone)]
pub struct Sender {
    unique_id: Uuid,
    id: PeerId,
    pc: Arc<RTCPeerConnection>,
    dc: Option<Arc<RTCDataChannel>>,
    #[allow(dead_code)]
    peer_addr: Addr<Peer>,
    last_offer_id: u64,
}

impl Connector for Sender {
    fn get_pc(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.pc)
    }
}

impl DataChannel for Sender {
    fn set_dc(&mut self, dc: Arc<RTCDataChannel>) {
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
        Ok(Self {
            unique_id: Uuid::new_v4(),
            id,
            pc,
            dc: None,
            peer_addr,
            last_offer_id: 0,
        })
    }

    pub(crate) async fn setup_offer(&mut self) -> MediaResult<String> {
        if let Err(e) = self
            .create_data_channel(self.peer_addr.clone(), ConnectorType::Sender)
            .await
        {
            return Err(MediaError::DataCannel(e.to_string()));
        }
        log::info!(
            "connect and create answer (Sender), peer_id={}, sender_id={}",
            self.id,
            self.unique_id
        );

        let offer = self.create_offer().await?;
        Ok(offer)
    }

    pub async fn add_media(&self, media: Media) -> MediaResult<()> {
        log::info!(
            "add track (Sender), peer_id={}, sender_id={}",
            self.id,
            self.unique_id
        );

        let track = Arc::new(TrackLocalStaticRTP::new(
            media.capability.clone(),
            media.id.to_string(),
            media.stream_id.clone(),
        ));

        let pc = self.get_pc();

        if let Err(e) = pc
            .add_track(Arc::clone(&track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
        {
            return Err(e.into());
        };

        media.subscribe(track).await;
        Ok(())
    }

    pub async fn remove_track(&self, track_id: String) -> MediaResult<()> {
        log::info!(
            "remove track (Sender) peer_id={}, sender_id={}",
            self.id,
            self.unique_id
        );
        for sender in self.pc.get_senders().await.iter() {
            if let Some(sender_track) = sender.track().await {
                if sender_track.id() == track_id {
                    if let Err(e) = self.pc.remove_track(sender).await {
                        return Err(e.into());
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn send_signaling_offer(&mut self) -> MediaResult<()> {
        log::info!("signaling (Sender) offer, peer_id={}", self.id);
        let peer_id = self.id.clone();
        if self.get_dc().is_none() {
            log::warn!(
                "data channel (Sender) is not initialized in sender of peer_id={}",
                peer_id
            );
            return Err(MediaError::DataCannel(
                "Data channel (Sender) is not initialized".to_string(),
            ));
        };

        let offer = self.create_offer().await?;
        let offer_id = self.next_offer_id();

        let msg = DataChannelMsg::OfferMsg(SdpMsgData {
            number: offer_id,
            sdp: offer,
        });

        match self.send_dcm(msg).await {
            Ok(_) => Ok(()),
            Err(e) => Err(MediaError::Renegotiation(format!("{:?}", e))),
        }
    }

    pub async fn on_signaling_answer(&mut self, msg: SdpMsgData) -> MediaResult<()> {
        log::info!("receive (Sender) signaling answer, peer_id={}", self.id);
        let peer_id = self.id.clone();
        if self.is_answer_stale(msg.number) {
            log::info!("Signal answer (Sender) is stale, peer_id={}", peer_id);
            return Ok(());
        }
        self.set_answer(msg.sdp.as_str()).await
    }

    pub fn next_offer_id(&mut self) -> u64 {
        self.last_offer_id += 1;
        self.last_offer_id
    }

    pub fn is_answer_stale(&self, answer_id: u64) -> bool {
        answer_id < self.last_offer_id
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
