use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::peer::Peer;
use actix::{Addr, Message};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
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
    #[allow(dead_code)]
    pub fn to_json(&self) -> anyhow::Result<String> {
        let json = serde_json::to_string(self)?; // serialize zu JSON-Bytes
        Ok(json)
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

#[allow(dead_code)]
pub trait DataChannel: Connector {
    fn initialize_data_channel(&mut self, peer_addr: Addr<Peer>, kind: ConnectorType) {
        let peer_connection = self.get_pc();
        {
            // Set a data channel handler so that we can receive data
            peer_connection.on_data_channel(Box::new(move |dc| {
                let d_label = dc.label().to_owned();
                let d_id = dc.id();
                log::info!("New DataChannel {d_label} {d_id}");
                peer_addr.do_send(OnDataChannel {
                    kind,
                    dc: Arc::clone(&dc),
                });
                let peer_addr_clone = peer_addr.clone();
                Box::pin(async move {
                    dc.on_open(Box::new(move || Box::pin(async move {})));

                    dc.on_message(Box::new(move |dcm| {
                        //msg.is_string
                        let addr = peer_addr_clone.clone();
                        let msg = DataChannelMsg::from_data_channel_message(&dcm).unwrap();
                        Box::pin(async move {
                            addr.do_send(msg);
                        })
                    }));
                })
            }));
        }
    }

    async fn send_dcm(&self, msg: DataChannelMsg) -> anyhow::Result<()> {
        let Some(dc) = self.get_dc() else {
            return Ok(());
        };
        let dcm = msg.to_json()?;
        let _ = dc.send_text(dcm).await.map_err(|e| Error::from(e))?;
        Ok(())
    }

    async fn send_dcm_bin(&self, msg: DataChannelMsg) -> anyhow::Result<()> {
        let Some(dc) = self.get_dc() else {
            return Ok(());
        };
        let dcm = msg.to_bin()?;
        let _ = dc.send(&dcm).await.map_err(|e| Error::from(e))?;
        Ok(())
    }

    fn set_dc(&mut self, dc: Arc<RTCDataChannel>);
    fn get_dc(&self) -> Option<Arc<RTCDataChannel>>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OnDataChannel {
    pub kind: ConnectorType,
    #[allow(dead_code)]
    pub dc: Arc<RTCDataChannel>,
}
