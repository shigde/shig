use crate::sfu::lobby::Lobby;
use crate::sfu::media::track_info::InboundTrackInfo;
use crate::sfu::media::{AddMedia, Media, RemoveMedia, RtpCodecType};
use crate::sfu::peer::PeerId;
use crate::sfu::rtc::endpoint::{Endpoint, EndpointId};
use crate::sfu::rtc::sdp::parse_offered_track_info;
use crate::sfu::rtc::{RtcError, RtcResult};
use actix::Addr;
use rtc::rtp_transceiver::rtp_sender::RTCRtpCodec;
use std::sync::Arc;
use tokio::sync::broadcast;
use webrtc::media_stream::track_remote::{TrackRemote, TrackRemoteEvent};
use webrtc::peer_connection::RTCSessionDescription;

/// Actor-owned connection that receives media from a peer.
pub struct PublishEndpoint {
    endpoint: Endpoint,
    offered_track_infos: Vec<InboundTrackInfo>,
}

impl PublishEndpoint {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            offered_track_infos: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn id(&self) -> &EndpointId {
        &self.endpoint.id
    }

    pub async fn accept_offer(&mut self, sdp: String) -> RtcResult<String> {
        self.offered_track_infos = parse_offered_track_info(&sdp)
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))?;

        let offer = RTCSessionDescription::offer(sdp)
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))?;
        self.endpoint
            .peer_connection
            .set_remote_description(offer)
            .await
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))?;

        let answer = self
            .endpoint
            .peer_connection
            .create_answer(None)
            .await
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))?;

        let answer = self
            .endpoint
            .set_local_description_and_gather(answer)
            .await
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))?;

        Ok(answer.sdp)
    }

    pub async fn receive_track(
        &mut self,
        peer_id: PeerId,
        lobby_addr: Addr<Lobby>,
        track: Arc<dyn TrackRemote>,
    ) {
        let kind = track.kind().await;
        let track_id = track.track_id().await;
        let stream_id = track.stream_id().await;
        let track_info = self.take_track_info(kind, &track_id, &stream_id);
        log::info!(
            "receive publish track, peer_id={}, endpoint_id={}, kind={:?}, track_id={}, stream_id={}, mid={}",
            peer_id,
            self.endpoint.id,
            kind,
            track_id,
            stream_id,
            track_info.mid
        );

        tokio::spawn(async move {
            let mut media = None;
            let (relay_rtp_tx, _relay_rtp_rx) = broadcast::channel(8192);
            while let Some(event) = track.poll().await {
                match event {
                    TrackRemoteEvent::OnRtpPacket(packet) => {
                        if media.is_none() {
                            let codec = track
                                .codec(packet.header.ssrc)
                                .await
                                .unwrap_or_else(RTCRtpCodec::default);
                            let source_media = Media::new(
                                peer_id.clone(),
                                track_info.mid.clone(),
                                track_id.clone(),
                                stream_id.clone(),
                                track_info.header_extensions.clone(),
                                codec,
                                kind,
                                packet.header.ssrc,
                                Some(Arc::clone(&track)),
                                relay_rtp_tx.clone(),
                                track_info.muted,
                                track_info.purpose.clone(),
                                track_info.info.clone(),
                                packet.header.payload_type,
                            );
                            lobby_addr.do_send(AddMedia {
                                media: source_media.clone(),
                            });
                            media = Some(source_media);
                        }

                        if let Some(media) = &media {
                            let _ = media.relay_rtp_tx.send(Arc::new(packet.clone()));
                            media.forward_rtp(packet).await;
                        }
                    }
                    TrackRemoteEvent::OnEnded | TrackRemoteEvent::OnError => {
                        if let Some(media) = &media {
                            lobby_addr.do_send(RemoveMedia {
                                media_id: media.id.clone(),
                            });
                        }
                        break;
                    }
                    _ => {}
                }
            }
        });
    }

    fn take_track_info(
        &mut self,
        kind: RtpCodecType,
        track_id: &str,
        stream_id: &str,
    ) -> InboundTrackInfo {
        if let Some(index) = self.offered_track_infos.iter().position(|track_info| {
            track_info.kind == kind
                && track_info.source_track_id == track_id
                && (track_info.source_stream_id.is_empty()
                    || track_info.source_stream_id == stream_id)
        }) {
            return self.offered_track_infos.remove(index);
        }

        let matching_kind = self
            .offered_track_infos
            .iter()
            .enumerate()
            .filter(|(_, track_info)| track_info.kind == kind)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        if matching_kind.len() == 1 {
            let index = matching_kind[0];
            let track_info = &self.offered_track_infos[index];
            log::warn!(
                "using single-track metadata fallback, endpoint_id={}, kind={:?}, track_id={}, stream_id={}, offered_track_id={}, offered_stream_id={}",
                self.endpoint.id,
                kind,
                track_id,
                stream_id,
                track_info.source_track_id,
                track_info.source_stream_id,
            );
            return self.offered_track_infos.remove(index);
        }

        log::warn!(
            "missing unambiguous offered track metadata, endpoint_id={}, kind={:?}, track_id={}, stream_id={}, candidates={}",
            self.endpoint.id,
            kind,
            track_id,
            stream_id,
            matching_kind.len(),
        );
        InboundTrackInfo::new(kind)
    }

    pub async fn shutdown(&self) -> RtcResult<()> {
        self.endpoint
            .peer_connection
            .close()
            .await
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))
    }
}
