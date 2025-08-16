use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::media::error::MediaResult;
use crate::sfu::peer::Peer;
use actix::Addr;
use std::sync::Arc;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_local::TrackLocal;

pub struct Sender {
    id: String,
    pc: Arc<RTCPeerConnection>,
    peer_addr: Addr<Peer>,
}

impl Connector for Sender {
    fn get_pc(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.pc)
    }
}

impl Sender {
    pub(crate) async fn new(id: String, peer_addr: Addr<Peer>) -> MediaResult<Self> {
        let pc =
            Self::create_connection(id.clone(), peer_addr.clone(), ConnectorType::Sender).await?;
        Ok(Self { id, pc, peer_addr })
    }

    pub(crate) async fn connect(&self, sdp_offer: &str) -> MediaResult<String> {
        let answer = self.create_answer(sdp_offer).await?;
        Ok(answer)
    }

    pub async fn add_track(&self, track: Arc<dyn TrackLocal + Send + Sync>) -> MediaResult<()> {
        if let Err(e) = self.pc.add_track(track).await {
            return Err(e.into());
        };
        Ok(())
    }
}
