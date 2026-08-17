//! Lobby-owned relay actor boundary.
//!
//! The previous media-track integration is intentionally not compiled while the
//! endpoint-based RTC core is being built. Ownership and lifecycle stay in place
//! so the relay can later consume the new RTC output without moving actors again.

pub mod actor {
    use crate::relay::state::RelayState;
    use crate::sfu::endpoint::EndpointId;
    use crate::worker::manager::WorkerManager;
    use actix::{Actor, ActorContext, Addr, Context, Handler, Message};
    use moq_relay::AuthToken;
    use std::collections::HashSet;

    pub struct RelayActor {
        stream_uuid: String,
        _relay_state: RelayState,
        _worker_manager: Addr<WorkerManager>,
        sources: HashSet<EndpointId>,
        streaming: bool,
    }

    impl RelayActor {
        pub fn new(
            relay_state: RelayState,
            worker_manager: Addr<WorkerManager>,
            stream_uuid: String,
        ) -> Self {
            Self {
                stream_uuid,
                _relay_state: relay_state,
                _worker_manager: worker_manager,
                sources: HashSet::new(),
                streaming: false,
            }
        }
    }

    impl Actor for RelayActor {
        type Context = Context<Self>;

        fn started(&mut self, _ctx: &mut Self::Context) {
            log::info!("relay actor stream_uuid={} is alive", self.stream_uuid);
        }
    }

    #[derive(Message)]
    #[rtype(result = "()")]
    pub struct RelayShutdown;

    #[derive(Message)]
    #[rtype(result = "()")]
    pub struct AttachRelaySource {
        pub endpoint_id: EndpointId,
    }

    #[derive(Message)]
    #[rtype(result = "()")]
    pub struct DetachRelaySource {
        pub endpoint_id: EndpointId,
    }

    #[derive(Message)]
    #[rtype(result = "Result<(), String>")]
    pub struct StartRelayMediaStream {
        pub auth_token: AuthToken,
    }

    #[derive(Message)]
    #[rtype(result = "Result<(), String>")]
    pub struct StopRelayMediaStream;

    impl Handler<AttachRelaySource> for RelayActor {
        type Result = ();

        fn handle(&mut self, message: AttachRelaySource, _ctx: &mut Self::Context) {
            self.sources.insert(message.endpoint_id);
        }
    }

    impl Handler<DetachRelaySource> for RelayActor {
        type Result = ();

        fn handle(&mut self, message: DetachRelaySource, _ctx: &mut Self::Context) {
            self.sources.remove(&message.endpoint_id);
        }
    }

    impl Handler<StartRelayMediaStream> for RelayActor {
        type Result = Result<(), String>;

        fn handle(
            &mut self,
            message: StartRelayMediaStream,
            _ctx: &mut Self::Context,
        ) -> Self::Result {
            if self.streaming {
                return Err("relay media stream is already started".to_owned());
            }
            if self.sources.is_empty() {
                return Err("relay has no sending peer endpoint".to_owned());
            }
            let _auth_token = message.auth_token;
            self.streaming = true;
            Ok(())
        }
    }

    impl Handler<StopRelayMediaStream> for RelayActor {
        type Result = Result<(), String>;

        fn handle(
            &mut self,
            _message: StopRelayMediaStream,
            _ctx: &mut Self::Context,
        ) -> Self::Result {
            self.streaming = false;
            Ok(())
        }
    }

    impl Handler<RelayShutdown> for RelayActor {
        type Result = ();

        fn handle(&mut self, _message: RelayShutdown, ctx: &mut Self::Context) {
            self.streaming = false;
            self.sources.clear();
            ctx.stop();
        }
    }
}
