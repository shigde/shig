use crate::sfu::media::error::{MediaError, MediaResult};
use crate::sfu::media::message::MediaMessage;
use crate::sfu::peer::Peer;
use actix::Addr;
use std::sync::Arc;
use webrtc::api::APIBuilder;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

#[derive(Clone, Copy)]
pub enum ConnectorType {
    Sender,
    Receiver,
}

pub trait Connector {
    async fn create_connection(
        id: String,
        peer_addr: Addr<Peer>,
        conn_type: ConnectorType,
    ) -> MediaResult<Arc<RTCPeerConnection>> {
        let api = APIBuilder::new().build();
        let config = RTCConfiguration {
            ice_servers: vec![], // Optional: add STUN/TURN if NAT is required.
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
                log::info!("Peer Connection State has changed: {s}, id: {id_clone}");
                if s == RTCPeerConnectionState::Connected {
                    let _ = addr_clone.do_send(MediaMessage::Connected(conn_type_clone));
                } else if s == RTCPeerConnectionState::Failed {
                    // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                    // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                    // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                    let _ = addr_clone.do_send(MediaMessage::Disconnected(conn_type_clone));
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

        // 5) Wait for ICE gathering to finish before setting local description to include candidates
        //    webrtc-rs exposes a gathering_complete_promise() helper which returns a receiver to await.
        let mut gather_complete = pc.gathering_complete_promise().await;

        if let Err(e) = pc.set_local_description(answer).await {
            return Err(MediaError::WebRTC(e));
        }

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

    fn get_pc(&self) -> Arc<RTCPeerConnection>;
}
