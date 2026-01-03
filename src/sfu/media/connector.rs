use crate::sfu::media::error::{MediaError, MediaResult};
use crate::sfu::media::message::MediaMessage;
use crate::sfu::media::sdp::OfferedMid;
use crate::sfu::peer::{Peer, PeerId};
use actix::Addr;
use derive_more::Display;
use std::sync::Arc;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::policy::bundle_policy::RTCBundlePolicy;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;

#[derive(Clone, Copy, Display)]
pub enum ConnectorType {
    Sender,
    Receiver,
}
impl ConnectorType {
    pub fn channel_label(&self) -> &str {
        match self {
            ConnectorType::Sender => "whep",
            ConnectorType::Receiver => "whip",
        }
    }
}

pub trait Connector {
    async fn create_connection(
        id: PeerId,
        peer_addr: Addr<Peer>,
        conn_type: ConnectorType,
    ) -> MediaResult<Arc<RTCPeerConnection>> {
        let mut m = MediaEngine::default();

        m.register_default_codecs()?;

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut m)?;

        // Create the API object with the MediaEngine
        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();

        let config = RTCConfiguration {
            ice_servers: vec![], // Optional: add STUN/TURN if NAT is required.
            bundle_policy: RTCBundlePolicy::MaxBundle,
            ..Default::default()
        };

        let pc = match api.new_peer_connection(config).await {
            Ok(p) => Arc::new(p),
            Err(e) => {
                return Err(MediaError::WebRTC(e));
            }
        };

        {
            let addr_clone = peer_addr.clone();
            let conn_type_clone = conn_type.clone();
            let id_clone = id.clone();

            pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                log::info!(
                    "peer connection (type={}) state has changed to: {s}, peer_id={id_clone}",
                    conn_type_clone.clone()
                );
                if s == RTCPeerConnectionState::Connected {
                    let _ = addr_clone.try_send(MediaMessage::Connected(conn_type_clone));
                } else if s == RTCPeerConnectionState::Failed {
                    // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                    // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                    // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                    let _ = addr_clone.try_send(MediaMessage::Disconnected(conn_type_clone));
                }
                Box::pin(async move {})
            }));
        }

        Ok(pc)
    }

    async fn create_answer(&self, sdp_offer: &str) -> MediaResult<String> {
        let pc = self.get_pc();

        // 3) Set a remote offer
        let offer = match RTCSessionDescription::offer(sdp_offer.to_string()) {
            Ok(o) => o,
            Err(e) => {
                return Err(MediaError::WebRTC(e));
            }
        };

        if let Err(e) = pc.set_remote_description(offer).await {
            return Err(MediaError::WebRTC(e));
        }

        // 4) Create an answer
        let answer = match pc.create_answer(None).await {
            Ok(a) => a,
            Err(e) => {
                return Err(MediaError::WebRTC(e));
            }
        };

        if let Err(e) = pc.set_local_description(answer).await {
            return Err(MediaError::WebRTC(e));
        }

        // 5) Wait for ICE gathering to finish before setting local description to include candidates
        //    webrtc-rs exposes a gathering_complete_promise() helper which returns a receiver to await.
        let mut gather_complete = pc.gathering_complete_promise().await;
        // Block until ICE gather finished
        let _ = gather_complete.recv().await;

        // 6) Get final local SDP (contains ICE candidates)
        let local = match pc.local_description().await {
            Some(ld) => ld.sdp.clone(),
            None => {
                return Err(MediaError::SdpState("no local description".to_string()));
            }
        };

        Ok(local)
    }

    async fn create_offer(&self) -> MediaResult<String> {
        let pc = self.get_pc();

        let offer = pc.create_offer(None).await?;

        let mut gather_complete = pc.gathering_complete_promise().await;

        pc.set_local_description(offer.clone()).await?;

        // Block until ICE gather finished
        let _ = gather_complete.recv().await;

        let local = match pc.local_description().await {
            Some(ld) => ld.sdp.clone(),
            None => {
                return Err(MediaError::SdpState("no local description".to_string()));
            }
        };

        Ok(local)
    }

    async fn set_answer(&self, sdp_answer: &str) -> MediaResult<()> {
        let pc = self.get_pc();
        let answer = RTCSessionDescription::answer(sdp_answer.to_string())?;
        pc.set_remote_description(answer).await?;
        Ok(())
    }

    async fn add_answerer_transceivers(
        &mut self,
        pc: &Arc<RTCPeerConnection>,
        offered: &[OfferedMid],
    ) -> anyhow::Result<()> {
        for o in offered {
            pc.add_transceiver_from_kind(o.kind, None).await?;
        }
        Ok(())
    }

    fn get_pc(&self) -> Arc<RTCPeerConnection>;
}

pub async fn receiver_index(
    pc: Arc<RTCPeerConnection>,
    receiver: &Arc<RTCRtpReceiver>,
) -> anyhow::Result<usize> {
    for (i, t) in pc.get_transceivers().await.iter().enumerate() {
        let r = t.receiver().await;
        if Arc::ptr_eq(&r, receiver) {
            return Ok(i);
        }
    }
    anyhow::bail!("Receiver not found in transceivers");
}
