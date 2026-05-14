use crate::sfu::relay::error::{RelayError, RelayResult};
use crate::sfu::relay::message::{RelayFailed, StartRelayMediaStream, StopRelayMediaStream};
use crate::sfu::relay::port_allocator::PortAllocator;
use crate::worker::manager::WorkerManager;
use crate::worker::message::StartWorker;
use crate::worker::process::{Process, OUTPUT_BUFFER_SIZE};
use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, ResponseActFuture, WrapFuture};
use bytes::Bytes;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use webrtc::rtp::packet::Packet;
use webrtc::util::Marshal;
use crate::relay::state::RelayState;
use crate::sfu::relay::cmaf::publisher::HangAvPublisher;

pub struct RelayActor {
    stream_uuid: String,
    relay_state: RelayState,
    worker_manager: Addr<WorkerManager>,
    port_allocator: PortAllocator,
    cancel_token: Option<CancellationToken>,
}

impl Actor for RelayActor {
    type Context = actix::Context<Self>;
}

impl RelayActor {
    pub fn new(relay_state: RelayState,worker_manager: Addr<WorkerManager>, stream_uuid: String) -> Self {
        Self {
            relay_state,
            stream_uuid,
            worker_manager,
            port_allocator: PortAllocator::new(10000, 20000),
            cancel_token: None,
        }
    }
}

impl Handler<StartRelayMediaStream> for RelayActor {
    type Result = ResponseActFuture<Self, RelayResult<()>>;

    fn handle(&mut self, msg: StartRelayMediaStream, ctx: &mut Self::Context) -> Self::Result {
        if self.cancel_token.is_some() {
            return Box::pin(
                async { Err(RelayError::MediaStreamAlreadyStarted()) }.into_actor(self),
            );
        }

        let cancel_token = CancellationToken::new();
        self.cancel_token = Some(cancel_token.clone());
        let actor_addr = ctx.address();
        let publisher_cancel = cancel_token.clone();

        let Some(audio_port) = self.port_allocator.allocate_port() else {
            return Box::pin(async { Err(RelayError::PortAllocationError()) }.into_actor(self));
        };

        let Some(video_port) = self.port_allocator.allocate_port() else {
            self.port_allocator.release_port(audio_port);

            return Box::pin(async { Err(RelayError::PortAllocationError()) }.into_actor(self));
        };

        let sdp = Process::build_ffmpeg_sdp(audio_port, video_port);

        let worker_manager = self.worker_manager.clone();

        let Some(audio_track) = msg.media_stream.audio else {
            self.port_allocator.release_port(video_port);
            self.port_allocator.release_port(audio_port);
            return Box::pin(
                async { Err(RelayError::NoInputTrack("video".to_string())) }.into_actor(self),
            );
        };

        let Some(video_track) = msg.media_stream.video else {
            self.port_allocator.release_port(video_port);
            self.port_allocator.release_port(audio_port);
            return Box::pin(
                async { Err(RelayError::NoInputTrack("audio".to_string())) }.into_actor(self),
            );
        };

        let (video_tx, video_rx) = mpsc::channel::<Bytes>(OUTPUT_BUFFER_SIZE);
        let (audio_tx, audio_rx) = mpsc::channel::<Bytes>(OUTPUT_BUFFER_SIZE);

        let process =  match Process::build(&sdp, self.stream_uuid.as_str(), video_tx, audio_tx) {
            Ok(process) => process,
            Err(err) => {
                let err_str = format!("{:?}", err);
                self.port_allocator.release_port(video_port);
                self.port_allocator.release_port(audio_port);
                return Box::pin(
                    async { Err(RelayError::StartProcessFailed(err_str)) }.into_actor(self),
                );
            }
        };

        let token = msg.auth_token;

        let Some(origin) = self.relay_state.cluster.publisher(&token) else {
            return Box::pin(
                async { Err(RelayError::Unauthorized("Publisher not valid".to_string())) }.into_actor(self)
            );
        };

        let publisher = HangAvPublisher::new(
            origin,
            self.stream_uuid.clone(),
            video_rx,
            audio_rx,
        );

        Box::pin(
            async move {
                let worker_id = worker_manager
                    .send(StartWorker {
                        id: None,
                        process,
                    })
                    .await
                    .map_err(|e| RelayError::WorkerMailboxError(e.to_string()))?
                    .map_err(|e| RelayError::WorkerError(e))?;

                log::info!("started ffmpeg worker: {:?}", worker_id);

                let audio_addr = format!("127.0.0.1:{audio_port}")
                    .parse()
                    .map_err(|e| RelayError::InvalidAddress(e))?;

                let video_addr = format!("127.0.0.1:{video_port}")
                    .parse()
                    .map_err(|e| RelayError::InvalidAddress(e))?;

                let cancel_audio = cancel_token.clone();
                let actor_addr_audio = actor_addr.clone();
                tokio::spawn(async move {
                    if let Err(err) = forward_rtp_sender_to_udp(
                        audio_track.rtp_tx.subscribe(),
                        audio_addr,
                        cancel_audio.clone(),
                    )
                    .await
                    {
                        log::warn!("audio RTP forwarder stopped: {:?}", err);
                        cancel_audio.cancel();
                        actor_addr_audio.do_send(RelayFailed {
                            source: "AudioForwardRtpSenderToUdp",
                            error: format!("{:?}", err),
                        });
                    }
                });

                let cancel_video = cancel_token.clone();
                let actor_addr_video = actor_addr.clone();
                tokio::spawn(async move {
                    if let Err(err) = forward_rtp_sender_to_udp(
                        video_track.rtp_tx.subscribe(),
                        video_addr,
                        cancel_video.clone(),
                    )
                    .await
                    {
                        log::warn!("video RTP forwarder stopped: {:?}", err);
                        cancel_video.cancel();
                        actor_addr_video.do_send(RelayFailed {
                            source: "VideoForwardRtpSenderToUdp",
                            error: format!("{:?}", err),
                        });
                    }
                });

                // run hang av publisher
                tokio::spawn(async move {
                    if let Err(err) = publisher.run(publisher_cancel.clone()).await {
                        log::error!("Hang AV publisher failed: {:?}", err);
                        publisher_cancel.cancel();
                        actor_addr.do_send(RelayFailed {
                            source: "Publisher",
                            error: format!("{:?}", err),
                        });
                    }
                });

                Ok(())
            }
            .into_actor(self),
        )
    }
}

impl Handler<StopRelayMediaStream> for RelayActor {
    type Result = RelayResult<()>;

    fn handle(&mut self, _msg: StopRelayMediaStream, ctx: &mut Self::Context) -> Self::Result {
        let Some(cancel_token) = self.cancel_token.as_ref() else {
            return Err(RelayError::MediaStreamNotStarted());
        };

        cancel_token.cancel();
        self.cancel_token = None;
        ctx.stop();
        Ok(())
    }
}

impl Handler<RelayFailed> for RelayActor {
    type Result = ();

    fn handle(&mut self, msg: RelayFailed, ctx: &mut Self::Context) {
        log::error!("relay failed, stopping relay actor, source: {}, error: {}",msg.source, msg.error);
        ctx.address().do_send(StopRelayMediaStream {});
    }
}

pub async fn forward_rtp_sender_to_udp(
    mut rx: broadcast::Receiver<Arc<Packet>>,
    target: SocketAddr,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0").await?;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                log::info!("RTP forwarder cancelled: {}", target);
                return Ok(());
            }

            result = rx.recv() => {
                match result {
                    Ok(packet) => {
                        let raw = packet.marshal()?;
                        socket.send_to(&raw, target).await?;
                    }

                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        log::warn!("RTP forwarder lagged, skipped {} packets for {}", skipped, target);
                        continue;
                    }

                    Err(broadcast::error::RecvError::Closed) => {
                        log::info!("RTP sender closed for {}", target);
                        return Ok(());
                    }
                }
            }
        }
    }
}
