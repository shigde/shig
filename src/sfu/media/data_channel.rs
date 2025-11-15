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

    pub fn to_bin(&self) -> anyhow::Result<Bytes> {
        let json = serde_json::to_vec(self)?;
        Ok(Bytes::from(json))
    }

    #[allow(dead_code)]
    pub fn from_data_channel_message(dcm: &DataChannelMessage) -> anyhow::Result<DataChannelMsg> {
        if dcm.is_string {
            let msg: DataChannelMsg = serde_json::from_slice(&dcm.data)?;
            Ok(msg)
        } else {
            anyhow::bail!("Binary deserialization not implemented yet");
        }
    }

    #[allow(dead_code)]
    pub fn from_data_channel_message_bin(
        dcm: &DataChannelMessage,
    ) -> anyhow::Result<DataChannelMsg> {
        println!("######## RAW DATA: {:?}", &dcm.data);
        println!(
            "######## AS STRING: {:?}",
            String::from_utf8_lossy(&dcm.data)
        );
        println!("#########is_string: {:?}", dcm.is_string);

        if !dcm.is_string {
            let msg: DataChannelMsg = match serde_json::from_slice(&dcm.data) {
                Ok(msg) => msg,
                Err(err) => anyhow::bail!("Failed to deserialize data channel message: {err:?}"),
            };
            Ok(msg)
        } else {
            anyhow::bail!("Expected binary but got string");
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SdpMsgData {
    pub number: u64,
    pub sdp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MuteMsgData {
    pub mid: String,
    pub mute: bool,
}

pub trait DataChannel: Connector {
    async fn create_data_channel(
        &mut self,
        peer_addr: Addr<Peer>,
        kind: ConnectorType,
    ) -> anyhow::Result<()> {
        let peer_connection = self.get_pc();
        let data_channel = peer_connection
            .create_data_channel(kind.channel_label(), None)
            .await?;
        log::info!("created whep data channel, kind={kind}");

        attach_message_handler(&data_channel, peer_addr.clone(), kind.clone());
        attach_on_open(&data_channel, peer_addr.clone(), kind.clone());
        Ok(())
    }

    fn initialize_data_channel(&mut self, peer_addr: Addr<Peer>, kind: ConnectorType) {
        let peer_connection = self.get_pc();
        let peer_addr_clone = peer_addr.clone();
        let kind_clone = kind.clone();

        peer_connection.on_data_channel(Box::new(move |dc| {
            let kind = kind_clone.clone();
            let peer_addr = peer_addr_clone.clone();

            attach_message_handler(&dc, peer_addr.clone(), kind.clone());
            attach_on_open(&dc, peer_addr.clone(), kind.clone());

            Box::pin(async move {
                log::info!(
                    "New data channel announced (but not yet open): kind={}, label={}",
                    kind,
                    dc.label()
                );
            })
        }));
    }

    #[allow(dead_code)]
    async fn set_dc(&mut self, dc: Arc<RTCDataChannel>);
    fn get_dc(&self) -> Option<Arc<RTCDataChannel>>;
}

fn attach_message_handler(dc: &Arc<RTCDataChannel>, peer_addr: Addr<Peer>, kind: ConnectorType) {
    dc.on_message(Box::new(move |dcm: DataChannelMessage| {
        let addr = peer_addr.clone();
        Box::pin(async move {
            match DataChannelMsg::from_data_channel_message_bin(&dcm) {
                Ok(msg) => addr.do_send(msg),
                Err(err) => log::warn!(
                    "Data Channel, failed to parse message, kind={}: {err:?}",
                    kind
                ),
            }
        })
    }));
}

fn attach_on_open(dc: &Arc<RTCDataChannel>, peer_addr: Addr<Peer>, kind: ConnectorType) {
    let dc_open = Arc::clone(dc);
    let peer_addr_open = peer_addr.clone();
    let kind_open = kind.clone();

    dc.on_open(Box::new(move || {
        let dc_open = Arc::clone(&dc_open);
        let peer_addr_open = peer_addr_open.clone();
        let kind_open = kind_open.clone();

        Box::pin(async move {
            log::info!(
                "DataChannel opened: kind={}, label={}",
                kind_open,
                dc_open.label()
            );

            peer_addr_open.do_send(OnDataChannel {
                kind: kind_open,
                dc: dc_open.clone(),
            });
        })
    }));
}

pub trait DataChannelMessanger {
    #[allow(dead_code)]
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

    fn get_dc(&self) -> Option<Arc<RTCDataChannel>>;
    async fn set_dc(&mut self, dc: Arc<RTCDataChannel>);
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OnDataChannel {
    pub kind: ConnectorType,
    pub dc: Arc<RTCDataChannel>,
}
