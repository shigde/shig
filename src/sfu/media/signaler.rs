use crate::sfu::media::data_channel::{
    DataChannelMessanger, DataChannelMsg, MuteMsgData, SdpMsgData,
};
use crate::sfu::media::error::{MediaError, MediaResult};
use crate::sfu::peer::{Peer, PeerId};
use actix::Addr;
use std::sync::Arc;
use webrtc::data_channel::RTCDataChannel;

#[derive(Clone)]
pub struct Signaler {
    id: PeerId,
    // for signaling we use the data channel of the receiver
    signal_dc: Option<Arc<RTCDataChannel>>,
    #[allow(dead_code)]
    peer_addr: Addr<Peer>,
    last_offer_number: u64,
    offer_msg: Option<SdpMsgData>,
}

impl DataChannelMessanger for Signaler {
    fn get_dc(&self) -> Option<Arc<RTCDataChannel>> {
        self.signal_dc.clone()
    }

    async fn set_dc(&mut self, dc: Arc<RTCDataChannel>) {
        if self.signal_dc.is_some() {
            log::warn!(
                "signaling data channel for peer_id={} is already set",
                self.id
            );
            return;
        }
        log::info!("set signaling data channel for peer_id={}", self.id);

        if let Some(offer) = self.offer_msg.take() {
            log::info!("send saved offer for peer_id={}", self.id);
            let msg = DataChannelMsg::OfferMsg(offer);
            if let Err(err) = self.send_dcm_bin(msg).await {
                log::error!("failed to send offer: {}", err);
            }
        }

        self.signal_dc = Some(dc);
    }
}

impl Signaler {
    pub fn new(id: PeerId, peer_addr: Addr<Peer>) -> Self {
        log::info!("create signaler for peer_id={}", id);

        Self {
            id,
            signal_dc: None,
            peer_addr,
            last_offer_number: 0,
            offer_msg: None,
        }
    }

    pub async fn send_offer(&mut self, offer: String) -> MediaResult<()> {
        log::info!("signaling (for Sender) offer, peer_id={}", self.id);
        let peer_id = self.id.clone();
        let offer_id = self.next_offer_id();
        let msg = SdpMsgData {
            number: offer_id,
            sdp: offer,
        };

        if self.get_dc().is_none() {
            log::info!(
                "data channel (Sender) is already not initialized in sender of peer_id={}, save offer",
                peer_id
            );
            // we save offer for later use
            self.offer_msg = Some(msg);
            return Ok(());
        }

        match self.send_dcm_bin(DataChannelMsg::OfferMsg(msg)).await {
            Ok(_) => Ok(()),
            Err(e) => Err(MediaError::Renegotiation(format!("{:?}", e))),
        }
    }

    pub async fn send_answer(&mut self, answer: String, number: u64) -> MediaResult<()> {
        log::info!("signaling (for Sender) answer, peer_id={}", self.id);

        let msg = DataChannelMsg::OfferMsg(SdpMsgData {
            number,
            sdp: answer,
        });
        match self.send_dcm_bin(msg).await {
            Ok(_) => Ok(()),
            Err(e) => Err(MediaError::Renegotiation(format!("{:?}", e))),
        }
    }

    fn next_offer_id(&mut self) -> u64 {
        self.last_offer_number += 1;
        self.last_offer_number
    }

    pub fn is_answer_stale(&self, answer_id: u64) -> bool {
        answer_id < self.last_offer_number
    }

    pub async fn send_mute(&mut self, mid: &str, mute: bool) -> MediaResult<()> {
        log::info!("muting (for Sender), peer_id={}", self.id);

        let msg = DataChannelMsg::MuteMsg(MuteMsgData {
            mid: mid.to_string(),
            mute,
        });
        match self.send_dcm_bin(msg).await {
            Ok(_) => Ok(()),
            Err(e) => Err(MediaError::Renegotiation(format!("{:?}", e))),
        }
    }
}
