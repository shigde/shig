use crate::relay::state::RelayState;
use crate::sfu::relay::actor_supervisor::RelayActorSupervisor;
use crate::sfu::relay::error::{RelayError, RelayResult};
use crate::sfu::relay::message::{RelayFailed, StartRelayMediaStream, StopRelayMediaStream};
use crate::sfu::relay::port_allocator::PortAllocator;
use crate::sfu::relay::rtp_forwarder::{forward_rtp_sender_to_udp, RtpForwarderKind};
use crate::worker::manager::WorkerManager;
use crate::worker::message::StartWorker;
use crate::worker::process::{Process, OUTPUT_BUFFER_SIZE};
use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Handler, ResponseActFuture, WrapFuture,
};
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio::sync::watch;
use crate::sfu::relay::publisher::HangPublisher;

pub struct RelayActor {
    stream_uuid: String,
    relay_state: RelayState,
    worker_manager: Addr<WorkerManager>,
    port_allocator: PortAllocator,
    supervisor: Option<RelayActorSupervisor>,
}

impl Actor for RelayActor {
    type Context = actix::Context<Self>;
}

impl RelayActor {
    pub fn new(
        relay_state: RelayState,
        worker_manager: Addr<WorkerManager>,
        stream_uuid: String,
    ) -> Self {
        Self {
            relay_state,
            stream_uuid,
            worker_manager,
            port_allocator: PortAllocator::new(10000, 20000),
            supervisor: None,
        }
    }
}

impl Handler<StartRelayMediaStream> for RelayActor {
    type Result = ResponseActFuture<Self, RelayResult<()>>;

    fn handle(&mut self, msg: StartRelayMediaStream, ctx: &mut Self::Context) -> Self::Result {
        if self.supervisor.is_some() {
            return Box::pin(
                async { Err(RelayError::MediaStreamAlreadyStarted()) }.into_actor(self),
            );
        }

        let worker_manager = self.worker_manager.clone();
        let supervisor =
            RelayActorSupervisor::new(self.stream_uuid.clone(), worker_manager.clone());
        self.supervisor = Some(supervisor.clone());
        let actor_addr = ctx.address();

        let Some(audio_port) = self.port_allocator.allocate_port() else {
            return Box::pin(async { Err(RelayError::PortAllocationError()) }.into_actor(self));
        };

        let Some(video_port) = self.port_allocator.allocate_port() else {
            self.port_allocator.release_port(audio_port);

            return Box::pin(async { Err(RelayError::PortAllocationError()) }.into_actor(self));
        };

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
        let sdp = Process::build_ffmpeg_sdp(
            video_port,
            audio_port,
            video_track.payload_type,
            audio_track.payload_type,
            video_track.capability.sdp_fmtp_line,
        );

        // Wait points for ffmpeg worker to start and forwarder to start a pipeline
        // 1. av publisher
        // 2. ffmpeg worker
        // 3. forwarder start sending
        let (publisher_ready_tx, publisher_ready_rx) = watch::channel(false);
        let (ffmpeg_ready_tx, ffmpeg_ready_rx) = watch::channel(false);
        let (pkg_tx, pkg_rx) = mpsc::channel::<Bytes>(OUTPUT_BUFFER_SIZE);

        let process = match Process::build(
            &sdp,
            self.stream_uuid.as_str(),
            pkg_tx,
            ffmpeg_ready_tx,
            supervisor.process_stopped.clone(),
        ) {
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
                async { Err(RelayError::Unauthorized("Publisher not valid".to_string())) }
                    .into_actor(self),
            );
        };

        let publisher = HangPublisher::new(
            origin,
            self.stream_uuid.clone(),
            pkg_rx,
            publisher_ready_tx,
        );

        let publisher_cancel = supervisor.publisher.clone();
        let publisher_stopped = supervisor.publisher_stopped.clone();

        let forwarder = supervisor.forwarder.clone();
        Box::pin(
            async move {
                let actor_addr_relay = actor_addr.clone();
                tokio::spawn(async move {
                    if let Err(err) = publisher
                        .run(publisher_cancel.clone(), publisher_stopped.clone())
                        .await
                    {
                        log::error!("Hang AV publisher failed: {:?}", err);
                        publisher_cancel.cancel();
                        actor_addr_relay.do_send(RelayFailed {
                            source: "Publisher",
                            error: format!("{:?}", err),
                        });
                    }
                });

                let worker_id = worker_manager
                    .send(StartWorker { id: None, process })
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

                let cancel_audio = forwarder.clone();
                let actor_addr_audio = actor_addr.clone();
                let ffmpeg_ready_audio_rx = ffmpeg_ready_rx.clone();
                tokio::spawn(async move {
                    if let Err(err) = forward_rtp_sender_to_udp(
                        audio_track.rtp_tx.subscribe(),
                        audio_addr,
                        cancel_audio.clone(),
                        ffmpeg_ready_audio_rx,
                        RtpForwarderKind::Audio,
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

                let cancel_video = forwarder.clone();
                let actor_addr_video = actor_addr.clone();
                let ffmpeg_ready_video_rx = ffmpeg_ready_rx.clone();
                tokio::spawn(async move {
                    if let Err(err) = forward_rtp_sender_to_udp(
                        video_track.rtp_tx.subscribe(),
                        video_addr,
                        cancel_video.clone(),
                        ffmpeg_ready_video_rx,
                        RtpForwarderKind::Video,
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

                // start the supervisor
                tokio::spawn(async move {
                    supervisor.start(worker_id).await;
                });

                Ok(())
            }
            .into_actor(self),
        )
    }
}

impl Handler<StopRelayMediaStream> for RelayActor {
    type Result = ResponseActFuture<Self, RelayResult<()>>;

    fn handle(&mut self, _msg: StopRelayMediaStream, _ctx: &mut Self::Context) -> Self::Result {
        let Some(supervisor) = self.supervisor.as_ref() else {
            return Box::pin(async { Err(RelayError::MediaStreamNotStarted()) }.into_actor(self));
        };

        log::info!(
            "stopping relay media stream, stream_id={}",
            self.stream_uuid
        );
        supervisor.shutdown.cancel();
        let is_down = supervisor.is_down.clone();
        Box::pin(
            async move {
                is_down.cancelled().await;
                Ok(())
            }
            .into_actor(self)
            .map(|result, actor, _ctx| {
                actor.supervisor = None;
                log::info!(
                    "relay media stream is stopped, stream_id={}",
                    actor.stream_uuid
                );
                result
            }),
        )
    }
}

impl Handler<RelayFailed> for RelayActor {
    type Result = ();

    fn handle(&mut self, msg: RelayFailed, ctx: &mut Self::Context) {
        log::error!(
            "relay failed, stopping relay actor, source: {}, error: {}",
            msg.source,
            msg.error
        );
        ctx.address().do_send(StopRelayMediaStream {});
    }
}
