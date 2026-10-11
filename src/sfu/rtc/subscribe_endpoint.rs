use crate::sfu::media::track_info::OutboundTrackInfo;
use crate::sfu::media::{ActivateMedia, ForwardTarget, Media, MediaId};
use crate::sfu::peer::PeerId;
use crate::sfu::rtc::data_channel::SdpMsgData;
use crate::sfu::rtc::endpoint::{Endpoint, EndpointId};
use crate::sfu::rtc::header_extension_translator::RtpHeaderExtensionTranslator;
use crate::sfu::rtc::sdp::set_track_info;
use crate::sfu::rtc::{RtcError, RtcResult};
use rtc::rtp_transceiver::rtp_sender::{RTCRtpCodingParameters, RTCRtpEncodingParameters};
use std::collections::HashMap;
use std::sync::Arc;
use webrtc::data_channel::DataChannel;
use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;
use webrtc::media_stream::track_local::TrackLocal;
use webrtc::media_stream::MediaStreamTrack;
use webrtc::peer_connection::RTCSessionDescription;
use webrtc::rtp_transceiver::{RTCRtpTransceiverDirection, RTCRtpTransceiverInit};

struct PendingSubscription {
    media: Media,
    track: Arc<TrackLocalStaticRTP>,
}

/// Actor-owned connection that sends subscribed media to a peer.
pub struct SubscribeEndpoint {
    endpoint: Endpoint,
    sending_media: HashMap<MediaId, OutboundTrackInfo>,
    pending_subscriptions: HashMap<MediaId, PendingSubscription>,
    data_channel: Option<Arc<dyn DataChannel>>,
}

impl SubscribeEndpoint {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            sending_media: HashMap::new(),
            pending_subscriptions: HashMap::new(),
            data_channel: None,
        }
    }

    pub fn id(&self) -> &EndpointId {
        &self.endpoint.id
    }

    pub async fn add_media(&mut self, media: Media) -> RtcResult<()> {
        let send_encoding = RTCRtpEncodingParameters {
            rtp_coding_parameters: RTCRtpCodingParameters {
                ssrc: Some(media.ssrc),
                ..Default::default()
            },
            active: true,
            codec: media.capability.clone(),
            ..Default::default()
        };
        let track = Arc::new(TrackLocalStaticRTP::new(MediaStreamTrack::new(
            media.src_stream_id.clone(),
            media.id.to_string(),
            media.id.to_string(),
            media.kind,
            vec![send_encoding.clone()],
        )));

        let transceiver = self
            .endpoint
            .peer_connection
            .add_transceiver_from_track(
                Arc::clone(&track) as Arc<dyn TrackLocal>,
                Some(RTCRtpTransceiverInit {
                    direction: RTCRtpTransceiverDirection::Sendonly,
                    send_encodings: vec![send_encoding],
                    streams: vec![],
                }),
            )
            .await
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?;

        let msid = format!("{} {}", media.src_stream_id.clone(), media.id.clone());
        self.sending_media.insert(
            media.id.clone(),
            OutboundTrackInfo::new(
                msid,
                transceiver,
                media.purpose.clone(),
                media.muted(),
                media.info.clone(),
            ),
        );

        self.pending_subscriptions
            .insert(media.id.clone(), PendingSubscription { media, track });
        Ok(())
    }

    pub async fn remove_track(&mut self, media_id: MediaId) -> RtcResult<()> {
        let media_id_string = media_id.to_string();
        let senders = self.endpoint.peer_connection.get_senders().await;

        for sender in senders {
            let track = sender.track();
            if track.track_id().await == media_id_string {
                self.endpoint
                    .peer_connection
                    .remove_track(&sender)
                    .await
                    .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?;
                self.sending_media.remove(&media_id);
                self.pending_subscriptions.remove(&media_id);
                return Ok(());
            }
        }

        Ok(())
    }

    pub async fn create_offer(&mut self) -> RtcResult<String> {
        self.ensure_data_channel().await?;

        let offer = self
            .endpoint
            .peer_connection
            .create_offer(None)
            .await
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?;

        let offer = self
            .endpoint
            .set_local_description_and_gather(offer)
            .await
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?;

        let track_info = self.sending_media.values().collect();
        let offer = set_track_info(offer, track_info)
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?;

        Ok(offer.sdp)
    }

    /// The subscribe endpoint is the WHEP offerer, so it creates the control channel.
    /// This guarantees an application M-line even before any media is available.
    async fn ensure_data_channel(&mut self) -> RtcResult<()> {
        if self.data_channel.is_some() {
            return Ok(());
        }

        let channel = self
            .endpoint
            .peer_connection
            .create_data_channel(self.endpoint.id.as_kind().channel_label(), None)
            .await
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?;

        log::info!(
            "created subscribe data channel, endpoint_id={}",
            self.endpoint.id
        );
        self.data_channel = Some(channel);
        Ok(())
    }

    pub async fn accept_answer(
        &mut self,
        subscriber_peer_id: PeerId,
        sdp: String,
    ) -> RtcResult<Vec<ActivateMedia>> {
        let answer = RTCSessionDescription::answer(sdp)
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?;

        self.endpoint
            .peer_connection
            .set_remote_description(answer)
            .await
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?;

        let pending_subscriptions = std::mem::take(&mut self.pending_subscriptions);
        let mut activations = Vec::with_capacity(pending_subscriptions.len());
        for (media_id, pending) in pending_subscriptions {
            let target = match self
                .start_subscription(&subscriber_peer_id, &media_id, &pending)
                .await
            {
                Ok(target) => target,
                Err(err) => {
                    self.pending_subscriptions.insert(media_id, pending);
                    return Err(err);
                }
            };
            activations.push(ActivateMedia { media_id, target });
        }
        Ok(activations)
    }

    async fn start_subscription(
        &self,
        subscriber_peer_id: &PeerId,
        media_id: &MediaId,
        pending: &PendingSubscription,
    ) -> RtcResult<ForwardTarget> {
        let outbound = self.sending_media.get(media_id).ok_or_else(|| {
            RtcError::SubscribeEndpoint(format!("missing outbound media {media_id}"))
        })?;
        let target_mid = outbound
            .transceiver
            .mid()
            .await
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?
            .ok_or_else(|| {
                RtcError::SubscribeEndpoint(format!("missing negotiated MID for {media_id}"))
            })?;
        let sender = outbound
            .transceiver
            .sender()
            .await
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?
            .ok_or_else(|| {
                RtcError::SubscribeEndpoint(format!("missing RTP sender for {media_id}"))
            })?;
        let parameters = sender
            .get_parameters()
            .await
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?;
        let header_extensions = RtpHeaderExtensionTranslator::new(
            &pending.media.header_extensions,
            &parameters.rtp_parameters.header_extensions,
            target_mid,
        );

        Ok(ForwardTarget::new(
            subscriber_peer_id.clone(),
            Arc::clone(&pending.track),
            header_extensions,
        ))
    }

    pub async fn set_signal_answer(&mut self, msg: SdpMsgData) -> RtcResult<Vec<ActivateMedia>> {
        self.accept_answer(self.endpoint.id.as_peer_id(), msg.sdp)
            .await
    }

    pub async fn get_mid_and_mute(&mut self, media_id: MediaId, muted: bool) -> Option<String> {
        let info = self.sending_media.get_mut(&media_id)?;
        info.muted = muted;
        info.transceiver.mid().await.ok().flatten()
    }

    pub async fn shutdown(&self) -> RtcResult<()> {
        self.endpoint
            .peer_connection
            .close()
            .await
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))
    }
}
