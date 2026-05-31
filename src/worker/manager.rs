use crate::worker::error::WorkerError;
use crate::worker::message::{
    GetWorkerState, ListWorkers, RestartWorker, ShutdownWorkers, StartWorker, StopWorker,
    WorkerExited, WorkerHandle,
};
use crate::worker::process::Process;
use crate::worker::state::WorkerState;
use crate::worker::WorkerId;
use actix::{
    Actor, Addr, AsyncContext, Context, Handler, MessageResult, ResponseActFuture, WrapFuture,
};
use actix::{ActorContext, ActorFutureExt};
use std::collections::HashMap;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct WorkerManager {
    workers: HashMap<WorkerId, WorkerHandle>,
}

impl WorkerManager {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
        }
    }
}

impl Actor for WorkerManager {
    type Context = Context<Self>;
}

impl Handler<StartWorker> for WorkerManager {
    type Result = ResponseActFuture<Self, Result<WorkerId, WorkerError>>;

    fn handle(&mut self, msg: StartWorker, ctx: &mut Self::Context) -> Self::Result {
        let id = msg.id.unwrap_or_else(WorkerId::new);

        if self.workers.contains_key(&id) {
            return Box::pin(async { Err(WorkerError::AlreadyExists) }.into_actor(self));
        }

        let cancel_token = CancellationToken::new();

        self.workers.insert(
            id.clone(),
            WorkerHandle {
                state: WorkerState::Running,
                shutdown: Some(cancel_token.clone()),
                process: msg.process.clone(),
            },
        );

        spawn_supervisor(id.clone(), msg.process, cancel_token, ctx.address());

        Box::pin(async move { Ok(id) }.into_actor(self))
    }
}

impl Handler<StopWorker> for WorkerManager {
    type Result = Result<(), WorkerError>;

    fn handle(&mut self, msg: StopWorker, _: &mut Self::Context) -> Self::Result {
        let id = msg.id.clone();
        log::info!("stopping worker {}", id.0);
        let handle = self.workers.get_mut(&msg.id).ok_or(WorkerError::NotFound)?;

        if let Err(error) = handle.start_shutdown(id.clone()) {
            log::error!("failed to stop worker {}: {}", id.0, error);
        }

        Ok(())
    }
}

impl Handler<RestartWorker> for WorkerManager {
    type Result = ResponseActFuture<Self, Result<(), WorkerError>>;

    fn handle(&mut self, msg: RestartWorker, _ctx: &mut Self::Context) -> Self::Result {
        let id = msg.id.clone();

        let process = match self.workers.get_mut(&msg.id) {
            Some(h) => {
                if let Err(error) = h.start_shutdown(id.clone()) {
                    log::error!("failed to stop worker {}: {}", id.0, error);
                    return Box::pin(async { Err(WorkerError::NotFound) }.into_actor(self));
                }
                h.process.clone()
            },
            None => {
                return Box::pin(async { Err(WorkerError::NotFound) }.into_actor(self));
            }
        };

        Box::pin(
            async move {
                tokio::time::sleep(Duration::from_millis(300)).await;
                (id, process)
            }
            .into_actor(self)
            .map(move |(id, process), act, ctx| {
                let cancel_token = CancellationToken::new();

                act.workers.insert(
                    id.clone(),
                    WorkerHandle {
                        state: WorkerState::Running,
                        shutdown: Some(cancel_token.clone()),
                        process: process.clone(),
                    },
                );

                spawn_supervisor(id, process, cancel_token, ctx.address());

                Ok(())
            }),
        )
    }
}

impl Handler<GetWorkerState> for WorkerManager {
    type Result = Option<WorkerState>;

    fn handle(&mut self, msg: GetWorkerState, _: &mut Self::Context) -> Self::Result {
        self.workers.get(&msg.id).map(|h| h.state.clone())
    }
}

impl Handler<ListWorkers> for WorkerManager {
    type Result = MessageResult<ListWorkers>;

    fn handle(&mut self, _: ListWorkers, _: &mut Self::Context) -> Self::Result {
        MessageResult(
            self.workers
                .iter()
                .map(|(id, handle)| (id.clone(), handle.state.clone()))
                .collect(),
        )
    }
}

impl Handler<WorkerExited> for WorkerManager {
    type Result = ();

    fn handle(&mut self, msg: WorkerExited, _: &mut Self::Context) {
        if msg.success {
            log::info!("worker {} finished: {}", msg.id.0, msg.reason);
            self.workers.remove(&msg.id);
        } else {
            log::warn!("worker {} failed: {}", msg.id.0, msg.reason);
            if let Some(handle) = self.workers.get_mut(&msg.id) {
                handle.state = WorkerState::Failed(msg.reason);
            }
        }
    }
}

fn spawn_supervisor(
    id: WorkerId,
    mut process: Process,
    cancel_token: CancellationToken,
    manager: Addr<WorkerManager>,
) {
    actix_rt::spawn(async move {
        let stream_id = process.stream_id.clone();
        let result = process.run(cancel_token).await;

        match result {
            Ok(reason) => {
                log::info!("worker {} for stream={}, finished: {}", id.0, stream_id, reason);
                let _ = manager
                    .send(WorkerExited {
                        id,
                        success: true,
                        reason,
                    })
                    .await;
            }
            Err(err) => {
                log::error!("worker {} for stream={}, failed: {}", id.0, stream_id, err);
                let _ = manager
                    .send(WorkerExited {
                        id,
                        success: false,
                        reason: err.to_string(),
                    })
                    .await;
            }
        }
    });
}

impl Handler<ShutdownWorkers> for WorkerManager {
    type Result = ();

    fn handle(&mut self, _msg: ShutdownWorkers, ctx: &mut Context<Self>) {
        log::info!("stopping all workers");

        for (id, mut child) in self.workers.drain() {
            if let Err(error) = child.start_shutdown(id.clone()) {
                log::error!("failed to stop worker {}: {}", id.0, error);
            }
        }

        ctx.stop();
    }
}
