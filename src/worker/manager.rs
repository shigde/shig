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
use tokio::sync::oneshot;

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

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        self.workers.insert(
            id.clone(),
            WorkerHandle {
                state: WorkerState::Running,
                shutdown_tx: Some(shutdown_tx),
                process: msg.process.clone(),
            },
        );

        spawn_supervisor(id.clone(), msg.process, shutdown_rx, ctx.address());

        Box::pin(async move { Ok(id) }.into_actor(self))
    }
}

impl Handler<StopWorker> for WorkerManager {
    type Result = Result<(), WorkerError>;

    fn handle(&mut self, msg: StopWorker, _: &mut Self::Context) -> Self::Result {
        let handle = self.workers.get_mut(&msg.id).ok_or(WorkerError::NotFound)?;

        if let Some(tx) = handle.shutdown_tx.take() {
            let _ = tx.send(());
        }

        Ok(())
    }
}

impl Handler<RestartWorker> for WorkerManager {
    type Result = ResponseActFuture<Self, Result<(), WorkerError>>;

    fn handle(&mut self, msg: RestartWorker, _ctx: &mut Self::Context) -> Self::Result {
        let process = match self.workers.get(&msg.id) {
            Some(h) => h.process.clone(),
            None => {
                return Box::pin(async { Err(WorkerError::NotFound) }.into_actor(self));
            }
        };

        if let Some(handle) = self.workers.get_mut(&msg.id) {
            if let Some(tx) = handle.shutdown_tx.take() {
                let _ = tx.send(());
            }
        }

        let id = msg.id.clone();

        Box::pin(
            async move {
                tokio::time::sleep(Duration::from_millis(300)).await;
                (id, process)
            }
            .into_actor(self)
            .map(move |(id, process), act, ctx| {
                let (shutdown_tx, shutdown_rx) = oneshot::channel();

                act.workers.insert(
                    id.clone(),
                    WorkerHandle {
                        state: WorkerState::Running,
                        shutdown_tx: Some(shutdown_tx),
                        process: process.clone(),
                    },
                );

                spawn_supervisor(id, process, shutdown_rx, ctx.address());

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
    shutdown_rx: oneshot::Receiver<()>,
    manager: Addr<WorkerManager>,
) {
    actix_rt::spawn(async move {
        let result = process.run(shutdown_rx).await;

        match result {
            Ok(reason) => {
                let _ = manager
                    .send(WorkerExited {
                        id,
                        success: true,
                        reason,
                    })
                    .await;
            }
            Err(err) => {
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

        for (_id, child) in self.workers.drain() {
            let _ = child.start_kill();
        }

        ctx.stop();
    }
}
