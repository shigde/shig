use crate::sfu::peer::Peer;
use crate::sfu::rtc::EndpointId;
use actix::{Addr, Message};
use bytes::{Bytes, BytesMut};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use webrtc::data_channel::{
    DataChannel as WebRtcDataChannel, DataChannelEvent, RTCDataChannelMessage,
};

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[serde(tag = "type", content = "data")]
#[rtype(result = "()")]
pub enum DataChannelMsg {
    #[serde(rename = "1")]
    OfferMsg(SdpMsgData),
    #[serde(rename = "2")]
    AnswerMsg(SdpMsgData),
    #[serde(rename = "3")]
    MuteMsg(MuteMsgData),
    #[serde(rename = "4")]
    HostStatus(HostStatus),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostStatus {
    Present,
    Waiting,
    Closed,
}

impl DataChannelMsg {
    #[allow(dead_code)]
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn to_bin(&self) -> anyhow::Result<Bytes> {
        Ok(Bytes::from(serde_json::to_vec(self)?))
    }

    #[allow(dead_code)]
    pub fn from_data_channel_message(
        message: &RTCDataChannelMessage,
    ) -> anyhow::Result<DataChannelMsg> {
        if message.is_string {
            Ok(serde_json::from_slice(&message.data)?)
        } else {
            anyhow::bail!("Binary deserialization not implemented yet");
        }
    }

    pub fn from_data_channel_message_bin(
        message: &RTCDataChannelMessage,
    ) -> anyhow::Result<DataChannelMsg> {
        if !message.is_string {
            Ok(serde_json::from_slice(&message.data)?)
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MuteMsgData {
    pub mid: String,
    pub mute: bool,
}

pub(crate) async fn create_data_channel(
    peer_addr: Addr<Peer>,
    endpoint_id: EndpointId,
    peer_connection: Arc<dyn webrtc::peer_connection::PeerConnection>,
) -> anyhow::Result<(Arc<dyn WebRtcDataChannel>, DataChannelReader)> {
    let data_channel = peer_connection
        .create_data_channel(endpoint_id.as_kind().channel_label(), None)
        .await?;
    log::info!("created data channel, endpoint_id={endpoint_id}");
    let reader = start_data_channel_reader(data_channel.clone(), peer_addr, endpoint_id);
    Ok((data_channel, reader))
}

/// Owns the task that bridges one WebRTC data channel into its parent peer actor.
/// Dropping or cancelling the handle always stops the reader.
pub(crate) struct DataChannelReader {
    cancel: CancellationToken,
    endpoint_id: EndpointId,
    channel_id: usize,
}

impl DataChannelReader {
    pub(crate) fn cancel(self) {
        self.cancel.cancel();
    }

    pub(crate) fn matches(
        &self,
        endpoint_id: &EndpointId,
        data_channel: &Arc<dyn WebRtcDataChannel>,
    ) -> bool {
        self.endpoint_id == *endpoint_id && self.channel_id == data_channel.id()
    }
}

impl Drop for DataChannelReader {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

pub(crate) fn start_data_channel_reader(
    data_channel: Arc<dyn WebRtcDataChannel>,
    peer_addr: Addr<Peer>,
    endpoint_id: EndpointId,
) -> DataChannelReader {
    let cancel = CancellationToken::new();
    let reader_cancel = cancel.clone();
    let reader_endpoint_id = endpoint_id.clone();
    let channel_id = data_channel.id();

    tokio::spawn(async move {
        loop {
            let event = tokio::select! {
                _ = reader_cancel.cancelled() => break,
                event = data_channel.poll() => event,
            };

            let Some(event) = event else {
                log::info!("DataChannel event stream ended: endpoint_id={}", endpoint_id);
                peer_addr.do_send(OnDataChannel {
                    event: EventType::Closed,
                    endpoint_id: endpoint_id.clone(),
                    dc: data_channel.clone(),
                });
                break;
            };

            match event {
                DataChannelEvent::OnOpen => {
                    log::info!("DataChannel opened: endpoint_id={}", endpoint_id);
                    peer_addr.do_send(OnDataChannel {
                        event: EventType::Open,
                        endpoint_id: endpoint_id.clone(),
                        dc: data_channel.clone(),
                    });
                }
                DataChannelEvent::OnMessage(message) => {
                    match DataChannelMsg::from_data_channel_message_bin(&message) {
                        Ok(msg) => peer_addr.do_send(msg),
                        Err(err) => log::warn!(
                            "DataChannel failed to parse message, endpoint_id={}: {err:?}",
                            endpoint_id
                        ),
                    }
                }
                DataChannelEvent::OnClose => {
                    log::info!("DataChannel closed: endpoint_id={}", endpoint_id);
                    peer_addr.do_send(OnDataChannel {
                        event: EventType::Closed,
                        endpoint_id: endpoint_id.clone(),
                        dc: data_channel.clone(),
                    });
                    break;
                }
                DataChannelEvent::OnError => {
                    log::warn!("DataChannel error: endpoint_id={}", endpoint_id);
                    peer_addr.do_send(OnDataChannel {
                        event: EventType::Closed,
                        endpoint_id: endpoint_id.clone(),
                        dc: data_channel.clone(),
                    });
                    break;
                }
                _ => {}
            }
        }
    });

    DataChannelReader {
        cancel,
        endpoint_id: reader_endpoint_id,
        channel_id,
    }
}

#[derive(Clone, Copy, Display)]
pub enum EventType {
    Open,
    Closed,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OnDataChannel {
    pub event: EventType,
    pub endpoint_id: EndpointId,
    pub dc: Arc<dyn WebRtcDataChannel>,
}

pub(crate) fn bytes_to_send_buffer(bytes: Bytes) -> BytesMut {
    BytesMut::from(&bytes[..])
}
