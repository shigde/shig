use crate::sfu::media::data_channel::{DataChannelMsg, MuteMsgData, SdpMsgData};
use crate::sfu::media::error::{MediaError, MediaResult};
use crate::sfu::peer::PeerId;
use std::collections::VecDeque;
use std::mem;
use std::sync::Arc;
use webrtc::data_channel::data_channel_state::RTCDataChannelState;
use webrtc::data_channel::RTCDataChannel;
use webrtc::Error;

pub struct ControlDataChannel {
    pub id: PeerId,
    dc: Option<Arc<RTCDataChannel>>,

    // message buffer in case dc is not set
    queue: VecDeque<DataChannelMsg>,

    // we only need to save the last offer in case dc is not set
    last_offer_number: u64,
}

impl ControlDataChannel {
    pub fn new(id: PeerId) -> Self {
       Self {
            id,
            dc: None,
            queue: VecDeque::new(),
            last_offer_number: 0,
        }
    }

    pub async fn send_offer(&mut self, offer: String) -> MediaResult<()> {
        log::info!("signaling (for Sender) offer, peer_id={}", self.id);
        let offer_id = self.next_offer_id();
        let msg = SdpMsgData {
            number: offer_id,
            sdp: offer,
        };

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

    #[allow(dead_code)]
    async fn send_dcm(&self, msg: DataChannelMsg) -> anyhow::Result<()> {
        let Some(dc) = &self.dc else {
            return Ok(());
        };
        let dcm = msg.to_json()?;
        let _ = dc.send_text(dcm).await.map_err(|e| Error::from(e))?;
        Ok(())
    }

    async fn send_dcm_bin(&mut self, msg: DataChannelMsg) -> anyhow::Result<()> {
        let Some(dc) = &self.dc else {
            log::warn!("No data channel available do send");
            self.queue.push_back(msg.clone());
            return Err(anyhow::anyhow!("No data channel available do send"));
        };

        if dc.ready_state() != RTCDataChannelState::Open {
            log::warn!("Data channel not open, dc in state: {}", dc.ready_state());
            self.queue.push_back(msg.clone());
            return Err(anyhow::anyhow!("Data channel not open"));
        }

        let Ok(dcm) = msg.to_bin() else {
            log::warn!("Failed to serialize data channel message");
            return Err(anyhow::anyhow!("Failed to serialize data channel message"));
        };

        let _ = dc.send(&dcm).await.map_err(|e| Error::from(e))?;
        Ok(())
    }

    pub async fn set_dc(&mut self, dc: Arc<RTCDataChannel>) {
        if let Some(ref dc_arc) = self.dc {
            if dc_arc.id() == dc.id() {
                return;
            }
        }
        self.dc = Some(dc);
        // try to send queue
        self.send_queue_messages().await
    }

    pub async fn send_queue_messages(&mut self) {
        // reset queue
        let messages = mem::take(&mut self.queue);
        let current_offer = self.last_offer_number;
        for msg in messages {
            if let DataChannelMsg::OfferMsg(ref offer) = msg {
                if offer.number != current_offer {
                    continue;
                }
            }

            let _ = self.send_dcm_bin(msg).await;
        }
    }

    pub fn detach_channel(&mut self, dc: Arc<RTCDataChannel>) {
        if let Some(ref dc_arc) = self.dc {
            if dc_arc.id() == dc.id() {
                self.dc = None;
            }
        }
    }
}
