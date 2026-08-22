use super::lobby::{decode_local_ufrag, RtcLobbyId};
use crate::sfu::endpoint::RtcEndpointId;
use rtc::shared::FourTuple;
use rtc::shared::TaggedBytesMut;
use rtc::stun::attributes::ATTR_USERNAME;
use rtc::stun::message::{is_stun_message, Message};
use rtc::stun::textattrs::Username;
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
            return Some(*lobby_peer);
        }

        self.demux_stun_username(pkt)
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

        Some((rtc_lobby_id, endpoint_id))
    }
}
