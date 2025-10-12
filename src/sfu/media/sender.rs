use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::media::data_channel::DataChannel;
use crate::sfu::media::error::MediaResult;
use crate::sfu::peer::{Peer, PeerId};
use actix::Addr;
use std::sync::Arc;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_local::TrackLocal;

pub struct Sender {
    #[allow(dead_code)]
    id: PeerId,
    pc: Arc<RTCPeerConnection>,
    dc: Option<Arc<RTCDataChannel>>,
    #[allow(dead_code)]
    peer_addr: Addr<Peer>,
}

impl Connector for Sender {
    fn get_pc(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.pc)
    }
}

impl DataChannel for Sender {
    fn set_dc(&mut self, dc: Arc<RTCDataChannel>) {
        self.dc = Some(dc);
    }

    fn get_dc(&self) -> Option<Arc<RTCDataChannel>> {
        self.dc.clone()
    }
}

impl Sender {
    pub(crate) async fn new(id: PeerId, peer_addr: Addr<Peer>) -> MediaResult<Self> {
        let pc =
            Self::create_connection(id.clone(), peer_addr.clone(), ConnectorType::Sender).await?;
        Ok(Self {
            id,
            pc,
            dc: None,
            peer_addr,
        })
    }

    pub(crate) async fn connect(&mut self, sdp_offer: &str) -> MediaResult<String> {
        self.initialize_data_channel(self.peer_addr.clone(), ConnectorType::Sender);
        let answer = self.create_answer(sdp_offer).await?;
        Ok(answer)
    }

    #[allow(dead_code)]
    pub async fn add_track(&self, track: Arc<dyn TrackLocal + Send + Sync>) -> MediaResult<()> {
        if let Err(e) = self.pc.add_track(track).await {
            return Err(e.into());
        };
        Ok(())
    }
}
