use crate::sfu::endpoint::{EndpointId, EndpointKind, RtcEndpointId};
use crate::sfu::peer::PeerId;
use bytes::BytesMut;
use rtc::data_channel::RTCDataChannelId;
use rtc::peer_connection::sdp::RTCSessionDescription;
use rtc::shared::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) enum ControlMessage {
    Answer {
        request_id: u64,
        sdp: RTCSessionDescription,
    },
    Metadata(ControlMetadata),
}

#[derive(Debug, Clone)]
pub(crate) enum ControlMetadata {
    Mute { mid: String, mute: bool },
}

#[derive(Debug, Default)]
pub(crate) struct ControlRouter {
    peers: HashMap<PeerId, PeerControl>,
}

#[derive(Debug, Default)]
struct PeerControl {
    publish_endpoint: Option<RtcEndpointId>,
    subscribe_endpoint: Option<RtcEndpointId>,
    channel_id: Option<RTCDataChannelId>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChannelMsg {
    id: u64,
    #[serde(deserialize_with = "deserialize_channel_type")]
    r#type: u8,
    data: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
struct SdpMsgData {
    number: u64,
    sdp: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct MuteMsgData {
    mid: String,
    mute: bool,
}

impl ControlRouter {
    pub(crate) fn register_endpoint(&mut self, endpoint_id: &EndpointId) {
        let peer = self.peers.entry(endpoint_id.peer_id().clone()).or_default();
        match endpoint_id.kind() {
            EndpointKind::Publish => peer.publish_endpoint = Some(endpoint_id.rtc_id()),
            EndpointKind::Subscribe => peer.subscribe_endpoint = Some(endpoint_id.rtc_id()),
        }
    }

    pub(crate) fn unregister_endpoint(&mut self, endpoint_id: &EndpointId) {
        let Some(peer) = self.peers.get_mut(endpoint_id.peer_id()) else {
            return;
        };
        match endpoint_id.kind() {
            EndpointKind::Publish => {
                if peer.publish_endpoint == Some(endpoint_id.rtc_id()) {
                    peer.publish_endpoint = None;
                    peer.channel_id = None;
                }
            }
            EndpointKind::Subscribe => {
                if peer.subscribe_endpoint == Some(endpoint_id.rtc_id()) {
                    peer.subscribe_endpoint = None;
                }
            }
        }
    }

    pub(crate) fn register_data_channel(
        &mut self,
        endpoint_id: &EndpointId,
        channel_id: RTCDataChannelId,
    ) {
        if endpoint_id.kind() != EndpointKind::Publish {
            return;
        }
        self.peers
            .entry(endpoint_id.peer_id().clone())
            .or_default()
            .channel_id = Some(channel_id);
    }

    pub(crate) fn control_channel_for_peer(
        &self,
        peer_id: &PeerId,
    ) -> Option<(RtcEndpointId, RTCDataChannelId)> {
        let peer = self.peers.get(peer_id)?;
        Some((peer.publish_endpoint?, peer.channel_id?))
    }

    pub(crate) fn subscription_endpoint_for_peer(&self, peer_id: &PeerId) -> Option<RtcEndpointId> {
        self.peers.get(peer_id)?.subscribe_endpoint
    }
}

pub(crate) fn parse_control_message(payload: &[u8]) -> Result<ControlMessage> {
    let msg: ChannelMsg = serde_json::from_slice(payload)
        .map_err(|err| Error::Other(format!("invalid control channel message: {err}")))?;

    match msg.r#type {
        2 => {
            let data: SdpMsgData = serde_json::from_value(msg.data)
                .map_err(|err| Error::Other(format!("invalid answer message: {err}")))?;
            Ok(ControlMessage::Answer {
                request_id: data.number,
                sdp: RTCSessionDescription::answer(data.sdp)?,
            })
        }
        3 => {
            let data: MuteMsgData = serde_json::from_value(msg.data)
                .map_err(|err| Error::Other(format!("invalid mute metadata message: {err}")))?;
            Ok(ControlMessage::Metadata(ControlMetadata::Mute {
                mid: data.mid,
                mute: data.mute,
            }))
        }
        other => Err(Error::Other(format!(
            "unsupported control channel message type: {other}"
        ))),
    }
}

pub(crate) fn serialize_offer_message(
    request_id: u64,
    sdp: &RTCSessionDescription,
) -> Result<BytesMut> {
    let msg = ChannelMsg {
        id: request_id,
        r#type: 1,
        data: serde_json::to_value(SdpMsgData {
            number: request_id,
            sdp: sdp.sdp.clone(),
        })
        .map_err(|err| Error::Other(format!("failed to serialize offer data: {err}")))?,
    };
    let json = serde_json::to_vec(&msg)
        .map_err(|err| Error::Other(format!("failed to serialize offer message: {err}")))?;
    Ok(BytesMut::from(json.as_slice()))
}

pub(crate) fn serialize_metadata_message(metadata: &ControlMetadata) -> Result<BytesMut> {
    let (id, data) = match metadata {
        ControlMetadata::Mute { mid, mute } => (
            0,
            serde_json::to_value(MuteMsgData {
                mid: mid.clone(),
                mute: *mute,
            })
            .map_err(|err| Error::Other(format!("failed to serialize mute metadata: {err}")))?,
        ),
    };
    let msg = ChannelMsg {
        id,
        r#type: 3,
        data,
    };
    let json = serde_json::to_vec(&msg)
        .map_err(|err| Error::Other(format!("failed to serialize metadata message: {err}")))?;
    Ok(BytesMut::from(json.as_slice()))
}

fn deserialize_channel_type<'de, D>(deserializer: D) -> std::result::Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(number) => number
            .as_u64()
            .and_then(|value| u8::try_from(value).ok())
            .ok_or_else(|| serde::de::Error::custom("channel type is out of range")),
        serde_json::Value::String(text) => text
            .parse::<u8>()
            .map_err(|err| serde::de::Error::custom(format!("invalid channel type: {err}"))),
        _ => Err(serde::de::Error::custom(
            "channel type must be a string or number",
        )),
    }
}
