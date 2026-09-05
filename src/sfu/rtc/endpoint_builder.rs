use crate::sfu::config::SfuConfig;
use crate::sfu::rtc::endpoint::{Endpoint, EndpointId};
use crate::sfu::rtc::error::{RtcError, RtcResult};
use crate::sfu::rtc::port_allocator::PortAllocator;
use std::net::SocketAddr;
use std::sync::Arc;
use webrtc::peer_connection::{
    register_default_interceptors, MediaEngine, PeerConnectionBuilder, PeerConnectionEventHandler,
    RTCConfigurationBuilder, RTCIceCandidateType, Registry, SettingEngineBuilder,
};

/// Builds WebRTC.rs peer connections for SFU peer endpoints.
///
/// This keeps WebRTC.rs setup details out of the actor layer. `PeerActor` owns
/// the resulting connections; this builder only creates them.
#[derive(Debug, Clone)]
pub struct EndpointBuilder {
    config: SfuConfig,
    dedicated_reactor_pool_size: usize,
    data_channel_send_buffer_limit: usize,
}

impl EndpointBuilder {
    pub fn from_sfu_config(config: &SfuConfig) -> Self {
        Self {
            config: config.clone(),
            dedicated_reactor_pool_size: 0,
            data_channel_send_buffer_limit: usize::MAX,
        }
    }

    pub fn with_dedicated_reactor_pool_size(mut self, size: usize) -> Self {
        self.dedicated_reactor_pool_size = size;
        self
    }

    pub fn with_data_channel_send_buffer_limit(mut self, bytes: usize) -> Self {
        self.data_channel_send_buffer_limit = bytes;
        self
    }

    pub async fn build(
        &self,
        id: EndpointId,
        handler: Arc<dyn PeerConnectionEventHandler>,
        port_allocator: &PortAllocator,
    ) -> RtcResult<Endpoint> {
        let lease = port_allocator
            .acquire()
            .ok_or_else(|| RtcError::Socket("no free UDP ports".into()))?;

        let bind_addr = SocketAddr::from(([0, 0, 0, 0], lease.port()));

        let mut media_engine = MediaEngine::default();
        media_engine
            .register_default_codecs()
            .map_err(|e| RtcError::EndpointBuild(e.to_string()))?;

        let mut setting_engine_builder = SettingEngineBuilder::default();

        if !self.config.advertised_ip.is_empty() {
            setting_engine_builder = setting_engine_builder.with_lite(true).with_nat_1to1_ips(
                vec![self.config.advertised_ip.clone()],
                RTCIceCandidateType::Host,
            );
        }

        let setting_engine = setting_engine_builder.build();

        let registry = register_default_interceptors(Registry::new(), &mut media_engine)
            .map_err(|e| RtcError::EndpointBuild(e.to_string()))?;

        let configuration = RTCConfigurationBuilder::default().build();

        let peer_connection = PeerConnectionBuilder::new()
            .with_udp_addrs(vec![bind_addr])
            .with_configuration(configuration)
            .with_media_engine(media_engine)
            .with_setting_engine(setting_engine)
            .with_interceptor_registry(registry)
            .with_handler(handler)
            .with_dedicated_reactor_pool_size(self.dedicated_reactor_pool_size)
            .with_data_channel_send_buffer_limit(self.data_channel_send_buffer_limit)
            .build()
            .await
            .map_err(|e| RtcError::EndpointBuild(e.to_string()))?;

        Ok(Endpoint::new(id, Arc::new(peer_connection), lease))
    }
}
