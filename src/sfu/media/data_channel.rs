use crate::sfu::peer::Peer;
use actix::{Addr, Message};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::Error;

#[derive(Serialize, Deserialize, Debug, Message)]
#[serde(tag = "type", content = "data")]
#[rtype(result = "()")]
pub enum DataChannelMsg {
    #[serde(rename = "1")]
    OfferMsg(SdpMsgData),
    #[serde(rename = "2")]
    AnswerMsg(SdpMsgData),
    #[serde(rename = "3")]
    MuteMsg(MuteMsgData),
}

impl DataChannelMsg {
    pub fn to_json(&self) -> anyhow::Result<String> {
        let json = serde_json::to_string(self)?; // serialize zu JSON-Bytes
        Ok(json)
    }

    pub fn to_bin(&self) -> anyhow::Result<Bytes> {
        let bin = bincode::serialize(self)?;
        Ok(Bytes::from(bin))
    }

    fn from_data_channel_message(dcm: &DataChannelMessage) -> anyhow::Result<DataChannelMsg> {
        if dcm.is_string {
            let msg: DataChannelMsg = serde_json::from_slice(&dcm.data)?;
            Ok(msg)
        } else {
            anyhow::bail!("Binary deserialization not implemented yet");
        }
    }

    fn from_data_channel_message_bin(dcm: &DataChannelMessage) -> anyhow::Result<DataChannelMsg> {
        if !dcm.is_string {
            let msg: DataChannelMsg = bincode::deserialize(&dcm.data)?;
            Ok(msg)
        } else {
            anyhow::bail!("Expected binary but got string");
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SdpMsgData {
    pub number: u32,
    pub sdp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MuteMsgData {
    pub mid: String,
    pub mute: bool,
}

const MESSAGE_SIZE: usize = 1500;

pub trait DataChannel {
    fn initialize_data_channel(&self, peer_addr: Addr<Peer>) {
        let peer_connection = self.get_pc();
        {
            // Set a data channel handler so that we can receive data
            peer_connection.on_data_channel(Box::new(move |dc| {
                let d_label = dc.label().to_owned();
                let d_id = dc.id();
                log::info!("New DataChannel {d_label} {d_id}");
                let peer_addr = peer_addr.clone();
                Box::pin(async move {
                    dc.on_open(Box::new(move || Box::pin(async move {})));

                    dc.on_message(Box::new(move |dcm| {
                        //msg.is_string
                        let peer_addr = peer_addr.clone();
                        let msg = DataChannelMsg::from_data_channel_message(&dcm).unwrap();
                        Box::pin(async move {
                            peer_addr.do_send(msg);
                        })
                    }));
                })
            }));
        }
    }

    async fn send_dcm(&self, msg: DataChannelMsg) -> anyhow::Result<()> {
        let dc = self.get_dc();
        let dcm = msg.to_json()?;
        let _ = dc.send_text(dcm).await.map_err(|e| Error::from(e))?;
        Ok(())
    }

    async fn send_dcm_bin(&self, msg: DataChannelMsg) -> anyhow::Result<()> {
        let dc = self.get_dc();
        let dcm = msg.to_bin()?;
        let _ = dc.send(&dcm).await.map_err(|e| Error::from(e))?;
        Ok(())
    }

    fn get_pc(&self) -> Arc<RTCPeerConnection>;
    fn set_dc(&self, dc: Arc<RTCDataChannel>);
    fn get_dc(&self) -> Arc<RTCDataChannel>;
}
