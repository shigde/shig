use crate::sfu::endpoint::{EndpointId, EndpointKind, RtcEndpointId};
use crate::sfu::peer::PeerId;
use bytes::BytesMut;
use rtc::data_channel::RTCDataChannelId;
use rtc::peer_connection::sdp::RTCSessionDescription;
use rtc::shared::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub(crate) enum ControlMessage {
    Answer {
        request_id: u64,
        sdp: RTCSessionDescription,
    },
    Metadata(ControlMetadata),
}

#[derive(Debug, Clone)]
pub(crate) enum ControlOutMessage {
    Offer {
        request_id: u64,
        sdp: RTCSessionDescription,
    },
    Metadata(ControlMetadata),
}

#[derive(Debug)]
pub(crate) struct ControlSend {
    pub(crate) endpoint_id: RtcEndpointId,
    pub(crate) channel_id: RTCDataChannelId,
    pub(crate) payload: BytesMut,
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
    queue: VecDeque<ControlOutMessage>,
    last_offer_request_id: Option<u64>,
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
    ) -> Result<Vec<ControlSend>> {
        if endpoint_id.kind() != EndpointKind::Publish {
            return Ok(Vec::new());
        }
        let peer = self
            .peers
            .entry(endpoint_id.peer_id().clone())
            .or_default();
        peer.channel_id = Some(channel_id);
        Self::flush_peer(peer)
    }

    pub(crate) fn enqueue_for_peer(
        &mut self,
        peer_id: &PeerId,
        message: ControlOutMessage,
    ) -> Result<Vec<ControlSend>> {
        let peer = self.peers.entry(peer_id.clone()).or_default();
        if let ControlOutMessage::Offer { request_id, .. } = &message {
            peer.last_offer_request_id = Some(*request_id);
            peer.queue
                .retain(|queued| !matches!(queued, ControlOutMessage::Offer { .. }));
        }
        peer.queue.push_back(message);
        Self::flush_peer(peer)
    }

    pub(crate) fn subscription_endpoint_for_peer(&self, peer_id: &PeerId) -> Option<RtcEndpointId> {
        self.peers.get(peer_id)?.subscribe_endpoint
    }

    pub(crate) fn is_stale_answer(&self, peer_id: &PeerId, request_id: u64) -> bool {
        self.peers
            .get(peer_id)
            .and_then(|peer| peer.last_offer_request_id)
            .is_some_and(|last_offer_request_id| request_id < last_offer_request_id)
    }

    fn flush_peer(peer: &mut PeerControl) -> Result<Vec<ControlSend>> {
        let Some(endpoint_id) = peer.publish_endpoint else {
            return Ok(Vec::new());
        };
        let Some(channel_id) = peer.channel_id else {
            return Ok(Vec::new());
        };

        let mut sends = Vec::with_capacity(peer.queue.len());
        while let Some(message) = peer.queue.pop_front() {
            let payload = serialize_control_out_message(&message)?;
            sends.push(ControlSend {
                endpoint_id,
                channel_id,
                payload,
            });
        }
        Ok(sends)
    }
}

fn serialize_control_out_message(message: &ControlOutMessage) -> Result<BytesMut> {
    match message {
        ControlOutMessage::Offer { request_id, sdp } => serialize_offer_message(*request_id, sdp),
        ControlOutMessage::Metadata(metadata) => serialize_metadata_message(metadata),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sfu::lobby::LobbyId;

    fn endpoint(rtc_id: RtcEndpointId, peer_id: &PeerId, kind: EndpointKind) -> EndpointId {
        EndpointId::new(
            rtc_id,
            LobbyId::new("lobby"),
            peer_id.clone(),
            kind,
        )
    }

    fn offer(request_id: u64) -> ControlOutMessage {
        let sdp = concat!(
            "v=0\r\n",
            "o=- 123456789 2 IN IP4 192.168.1.1\r\n",
            "s=-\r\n",
            "t=0 0\r\n",
            "m=video 9 UDP/TLS/RTP/SAVPF 96\r\n",
        );
        ControlOutMessage::Offer {
            request_id,
            sdp: RTCSessionDescription::offer(sdp.to_owned()).unwrap(),
        }
    }

    fn message_type(send: &ControlSend) -> u64 {
        serde_json::from_slice::<serde_json::Value>(send.payload.as_ref())
            .unwrap()
            .get("type")
            .unwrap()
            .as_u64()
            .unwrap()
    }

    fn message_number(send: &ControlSend) -> u64 {
        serde_json::from_slice::<serde_json::Value>(send.payload.as_ref())
            .unwrap()
            .get("data")
            .unwrap()
            .get("number")
            .unwrap()
            .as_u64()
            .unwrap()
    }

    #[test]
    fn queues_offer_until_publish_data_channel_is_registered() {
        let mut router = ControlRouter::default();
        let peer_id = PeerId::new("peer-1");
        let publish = endpoint(1, &peer_id, EndpointKind::Publish);
        let subscribe = endpoint(2, &peer_id, EndpointKind::Subscribe);

        router.register_endpoint(&publish);
        router.register_endpoint(&subscribe);

        let queued = router.enqueue_for_peer(&peer_id, offer(7)).unwrap();
        assert!(queued.is_empty());

        let sends = router.register_data_channel(&publish, 9).unwrap();
        assert_eq!(sends.len(), 1);
        assert_eq!(sends[0].endpoint_id, publish.rtc_id());
        assert_eq!(sends[0].channel_id, 9);
        assert_eq!(message_type(&sends[0]), 1);
        assert_eq!(message_number(&sends[0]), 7);
    }

    #[test]
    fn replaces_pending_offers_but_keeps_metadata_order() {
        let mut router = ControlRouter::default();
        let peer_id = PeerId::new("peer-1");
        let publish = endpoint(1, &peer_id, EndpointKind::Publish);

        router.register_endpoint(&publish);
        router.enqueue_for_peer(&peer_id, offer(1)).unwrap();
        router
            .enqueue_for_peer(
                &peer_id,
                ControlOutMessage::Metadata(ControlMetadata::Mute {
                    mid: "0".to_owned(),
                    mute: true,
                }),
            )
            .unwrap();
        router.enqueue_for_peer(&peer_id, offer(2)).unwrap();

        let sends = router.register_data_channel(&publish, 9).unwrap();
        assert_eq!(sends.len(), 2);
        assert_eq!(message_type(&sends[0]), 3);
        assert_eq!(message_type(&sends[1]), 1);
        assert_eq!(message_number(&sends[1]), 2);
    }

    #[test]
    fn detects_stale_answers_against_latest_offer() {
        let mut router = ControlRouter::default();
        let peer_id = PeerId::new("peer-1");

        router.enqueue_for_peer(&peer_id, offer(2)).unwrap();

        assert!(router.is_stale_answer(&peer_id, 1));
        assert!(!router.is_stale_answer(&peer_id, 2));
        assert!(!router.is_stale_answer(&peer_id, 3));
    }
}
