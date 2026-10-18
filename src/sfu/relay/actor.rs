use crate::relay::state::RelayState;
use crate::sfu::relay::error::{RelayError, RelayResult};
use crate::sfu::relay::message::{StartRelayMediaStream, StopRelayMediaStream};
use crate::sfu::relay::publisher::HangPublisher;
use crate::util::stop_guard::StopGuard;
use actix::{Actor, ActorFutureExt, AsyncContext, Handler, Message, ResponseActFuture, WrapFuture};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

struct RunningPublisher {
    cancel: CancellationToken,
    stopped: CancellationToken,
}

impl Drop for RunningPublisher {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

pub struct RelayActor {
    stream_uuid: String,
    relay_state: RelayState,
    running: Option<Arc<RunningPublisher>>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct PublisherFinished {
    run: Arc<RunningPublisher>,
    error: Option<String>,
}

impl Actor for RelayActor {
    type Context = actix::Context<Self>;
}

impl Drop for RelayActor {
    fn drop(&mut self) {
        if let Some(running) = &self.running {
            running.cancel.cancel();
        }
    }
}

impl RelayActor {
    pub fn new(relay_state: RelayState, stream_uuid: String) -> Self {
        Self {
            relay_state,
            stream_uuid,
            running: None,
        }
    }
}

impl Handler<StartRelayMediaStream> for RelayActor {
    type Result = ResponseActFuture<Self, RelayResult<()>>;

    fn handle(&mut self, msg: StartRelayMediaStream, ctx: &mut Self::Context) -> Self::Result {
        if self.running.is_some() {
            return Box::pin(
                async { Err(RelayError::MediaStreamAlreadyStarted()) }.into_actor(self),
            );
        }
        let Some(origin) = self.relay_state.cluster.publisher(&msg.auth_token) else {
            return Box::pin(
                async { Err(RelayError::Unauthorized("Publisher not valid".into())) }
                    .into_actor(self),
            );
        };
        let publisher = match HangPublisher::new(origin, self.stream_uuid.clone(), msg.media_stream)
        {
            Ok(publisher) => publisher,
            Err(err) => return Box::pin(async { Err(err) }.into_actor(self)),
        };
        let cancel = CancellationToken::new();
        let stopped = CancellationToken::new();
        let run = Arc::new(RunningPublisher {
            cancel: cancel.clone(),
            stopped: stopped.clone(),
        });
        self.running = Some(run.clone());
        let (ready_tx, ready_rx) = oneshot::channel();
        // A background task must not keep its owner actor alive after the lobby drops it.
        let addr = ctx.address().downgrade();
        tokio::spawn(async move {
            let _guard = StopGuard(stopped);
            let result = publisher.run(cancel, ready_tx).await;
            run.stopped.cancel();
            if let Some(addr) = addr.upgrade() {
                addr.do_send(PublisherFinished {
                    run,
                    error: result.err().map(|err| format!("{err:#}")),
                });
            }
        });
        Box::pin(
            async move {
                ready_rx.await.map_err(|_| {
                    RelayError::PublisherError("Publisher stopped during startup".into())
                })?
            }
            .into_actor(self),
        )
    }
}

impl Handler<StopRelayMediaStream> for RelayActor {
    type Result = ResponseActFuture<Self, RelayResult<()>>;

    fn handle(&mut self, _: StopRelayMediaStream, _: &mut Self::Context) -> Self::Result {
        let Some(running) = &self.running else {
            return Box::pin(async { Err(RelayError::MediaStreamNotStarted()) }.into_actor(self));
        };
        running.cancel.cancel();
        let running = running.clone();
        let stopped = running.stopped.clone();
        Box::pin(
            async move {
                stopped.cancelled().await;
                Ok(())
            }
            .into_actor(self)
            .map(move |result, actor, _| {
                if actor
                    .running
                    .as_ref()
                    .is_some_and(|current| Arc::ptr_eq(current, &running))
                {
                    actor.running = None;
                }
                result
            }),
        )
    }
}

impl Handler<PublisherFinished> for RelayActor {
    type Result = ();

    fn handle(&mut self, msg: PublisherFinished, _: &mut Self::Context) {
        if let Some(error) = msg.error {
            log::error!(
                "relay publisher failed, stream_id={}, error={error}",
                self.stream_uuid
            );
        }
        if self
            .running
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &msg.run))
        {
            self.running = None;
        }
    }
}
