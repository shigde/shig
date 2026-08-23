use super::lobby::{decode_local_ufrag, RtcLobbyId};
use crate::sfu::endpoint::RtcEndpointId;
use rtc::shared::FourTuple;
use rtc::shared::TaggedBytesMut;
use rtc::stun::attributes::ATTR_USERNAME;
use rtc::stun::message::{is_stun_message, Message};
use rtc::stun::textattrs::Username;
use log::trace;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub(crate) struct Demuxer {
    //TODO: handle expiry or eviction
    affinity: HashMap<FourTuple, (RtcLobbyId, RtcEndpointId)>,
    reverse: HashMap<(RtcLobbyId, RtcEndpointId), HashSet<FourTuple>>,
}

impl Demuxer {
    pub(crate) fn demux(&mut self, pkt: &TaggedBytesMut) -> Option<(RtcLobbyId, RtcEndpointId)> {
        let four_tuple = FourTuple::from(&pkt.transport);
        if let Some(lobby_peer) = self.affinity.get(&four_tuple) {
            trace!(
                "demux affinity hit kind={} local={} peer={} -> lobby={} endpoint={}",
                classify_rtc_payload(&pkt.message),
                pkt.transport.local_addr,
                pkt.transport.peer_addr,
                lobby_peer.0,
                lobby_peer.1
            );
            return Some(*lobby_peer);
        }

        let routed = self.demux_stun_username(pkt);
        if routed.is_none() {
            trace!(
                "demux miss kind={} local={} peer={}",
                classify_rtc_payload(&pkt.message),
                pkt.transport.local_addr,
                pkt.transport.peer_addr
            );
        }

        routed
    }

    fn demux_stun_username(&mut self, pkt: &TaggedBytesMut) -> Option<(RtcLobbyId, RtcEndpointId)> {
        if !is_stun_message(pkt.message.as_ref()) {
            return None;
        }

        let mut stun = Message::new();
        stun.unmarshal_binary(pkt.message.as_ref()).ok()?;

        // USERNAME = local_ufrag ":" remote_ufrag; the local half is what the SFU issued,
        // so it carries the lobby and peer (see `lobby::encode_local_ufrag`).
        let username = Username::get_from_as(&stun, ATTR_USERNAME).ok()?;
        let local_ufrag = username.text.split_once(':')?.0;
        let (rtc_lobby_id, endpoint_id) = decode_local_ufrag(local_ufrag)?;

        let four_tuple = pkt.transport.into();
        self.affinity
            .insert(four_tuple, (rtc_lobby_id, endpoint_id));
        self.reverse
            .entry((rtc_lobby_id, endpoint_id))
            .or_default()
            .insert(four_tuple);

        trace!(
            "demux stun bind local={} peer={} username={} -> lobby={} endpoint={}",
            pkt.transport.local_addr,
            pkt.transport.peer_addr,
            username.text,
            rtc_lobby_id,
            endpoint_id
        );

        Some((rtc_lobby_id, endpoint_id))
    }
}

fn classify_rtc_payload(payload: &[u8]) -> &'static str {
    let Some(first) = payload.first().copied() else {
        return "empty";
    };

    match first {
        0..=3 => "stun",
        20..=63 => "dtls",
        128..=191 => match payload.get(1).copied() {
            Some(192..=223) => "rtcp",
            Some(_) => "rtp",
            None => "rtp/rtcp",
        },
        _ => "unknown",
    }
}
