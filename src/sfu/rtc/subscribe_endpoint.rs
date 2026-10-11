use crate::sfu::media::track_info::OutboundTrackInfo;
use crate::sfu::media::{Media, MediaId};
use crate::sfu::rtc::data_channel::SdpMsgData;
use crate::sfu::rtc::endpoint::{Endpoint, EndpointId};
use crate::sfu::rtc::sdp::set_track_info;
use crate::sfu::rtc::{RtcError, RtcResult};
use std::collections::HashMap;
use std::sync::Arc;
use webrtc::data_channel::DataChannel;
use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;
use webrtc::media_stream::track_local::TrackLocal;
use webrtc::media_stream::MediaStreamTrack;
use webrtc::peer_connection::RTCSessionDescription;
use webrtc::rtp_transceiver::{
    RTCRtpTransceiverDirection, RTCRtpTransceiverInit,
};

/// Actor-owned connection that sends subscribed media to a peer.
pub struct SubscribeEndpoint {
    endpoint: Endpoint,
    sending_media: HashMap<MediaId, OutboundTrackInfo>,
    data_channel: Option<Arc<dyn DataChannel>>,
}

impl SubscribeEndpoint {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            sending_media: HashMap::new(),
            data_channel: None,
        }
    }

    pub fn id(&self) -> &EndpointId {
        &self.endpoint.id
    }

    pub async fn add_media(&mut self, media: Media) -> RtcResult<()> {
        let track = Arc::new(TrackLocalStaticRTP::new(MediaStreamTrack::new(
            media.src_stream_id.clone(),
            media.id.to_string(),
            media.id.to_string(),
            media.kind,
            vec![],
        )));

        let transceiver = self
            .endpoint
            .peer_connection
            .add_transceiver_from_track(
                Arc::clone(&track) as Arc<dyn TrackLocal>,
                Some(RTCRtpTransceiverInit {
                    direction: RTCRtpTransceiverDirection::Sendonly,
                    send_encodings: vec![],
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

        media.subscribe(track).await;
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

        log::info!("created subscribe data channel, endpoint_id={}", self.endpoint.id);
        self.data_channel = Some(channel);
        Ok(())
    }

    pub async fn accept_answer(&self, sdp: String) -> RtcResult<()> {
        let answer = RTCSessionDescription::answer(sdp)
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))?;

        self.endpoint
            .peer_connection
            .set_remote_description(answer)
            .await
            .map_err(|err| RtcError::SubscribeEndpoint(err.to_string()))
    }

    pub async fn set_signal_answer(&self, msg: SdpMsgData) -> RtcResult<()> {
        self.accept_answer(msg.sdp).await
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
