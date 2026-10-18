use crate::sfu::peer::PeerId;
use crate::sfu::rtc::data_channel::{
    bytes_to_send_buffer, DataChannelMsg, HostStatus, MuteMsgData, SdpMsgData,
};
use std::collections::VecDeque;
use std::mem;
use std::sync::Arc;
use webrtc::data_channel::{DataChannel as WebRtcDataChannel, RTCDataChannelState};

pub struct ControlDataChannel {
    pub id: PeerId,
    dc: Option<Arc<dyn WebRtcDataChannel>>,
    queue: VecDeque<DataChannelMsg>,
    last_offer_number: u64,
}

impl ControlDataChannel {
    pub async fn send_host_status(&mut self, status: HostStatus) -> anyhow::Result<()> {
        self.send_dcm_bin(DataChannelMsg::HostStatus(status)).await
    }

    pub fn new(id: PeerId) -> Self {
        Self {
            id,
            dc: None,
            queue: VecDeque::new(),
            last_offer_number: 0,
        }
    }

    pub async fn send_offer(&mut self, offer: String) -> anyhow::Result<()> {
        log::info!("signaling offer, peer_id={}", self.id);
        let offer_id = self.next_offer_id();
        let msg = SdpMsgData {
            number: offer_id,
            sdp: offer,
        };
        self.send_dcm_bin(DataChannelMsg::OfferMsg(msg)).await
    }

    #[allow(dead_code)]
    pub async fn send_answer(&mut self, answer: String, number: u64) -> anyhow::Result<()> {
        log::info!("signaling answer, peer_id={}", self.id);
        let msg = DataChannelMsg::AnswerMsg(SdpMsgData {
            number,
            sdp: answer,
        });
        self.send_dcm_bin(msg).await
    }

    fn next_offer_id(&mut self) -> u64 {
        self.last_offer_number += 1;
        self.last_offer_number
    }

    pub fn is_answer_stale(&self, answer_id: u64) -> bool {
        answer_id < self.last_offer_number
    }

    pub async fn send_mute(&mut self, mid: &str, mute: bool) -> anyhow::Result<()> {
        log::info!("muting remote media, peer_id={}", self.id);
        let msg = DataChannelMsg::MuteMsg(MuteMsgData {
            mid: mid.to_string(),
            mute,
        });
        self.send_dcm_bin(msg).await
    }

    #[allow(dead_code)]
    async fn send_dcm(&self, msg: DataChannelMsg) -> anyhow::Result<()> {
        let Some(dc) = &self.dc else {
            return Ok(());
        };
        let dcm = msg.to_json()?;
        dc.send_text(&dcm).await?;
        Ok(())
    }

    async fn send_dcm_bin(&mut self, msg: DataChannelMsg) -> anyhow::Result<()> {
        let Some(dc) = &self.dc else {
            log::warn!("No data channel available to send");
            self.queue.push_back(msg);
            anyhow::bail!("No data channel available to send");
        };

        let ready_state = dc.ready_state().await?;
        if ready_state != RTCDataChannelState::Open {
            log::warn!("Data channel not open, dc in state: {}", ready_state);
            self.queue.push_back(msg);
            anyhow::bail!("Data channel not open");
        }

        let dcm = msg.to_bin()?;
        dc.send(bytes_to_send_buffer(dcm)).await?;
        Ok(())
    }

    pub async fn set_dc(&mut self, dc: Arc<dyn WebRtcDataChannel>) {
        if let Some(ref dc_arc) = self.dc {
            if dc_arc.id() == dc.id() {
                return;
            }
        }
        self.dc = Some(dc);
        self.send_queue_messages().await
    }

    pub async fn send_queue_messages(&mut self) {
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

    pub fn detach_channel(&mut self, dc: Arc<dyn WebRtcDataChannel>) {
        if let Some(ref dc_arc) = self.dc {
            if dc_arc.id() == dc.id() {
                self.dc = None;
            }
        }
    }
}
