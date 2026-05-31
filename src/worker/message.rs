use crate::worker::error::{WorkerError, WorkerResult};
use crate::worker::process::Process;
use crate::worker::state::WorkerState;
use crate::worker::WorkerId;
use actix::Message;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct WorkerHandle {
    pub state: WorkerState,
    pub shutdown: Option<CancellationToken>,
    pub process: Process,
}

impl WorkerHandle {
    pub fn start_shutdown(&mut self, id: WorkerId) -> WorkerResult<()> {
        log::info!("start shutdown worker, worker_id={}", id.0);
        let stream_id = self.process.stream_id.clone();
        if let Some(token) = self.shutdown.take() {
            log::info!(
                "shutting down worker, worker_id={}, stream_id={}",
                id.0,
                stream_id
            );
            let _ = token.cancel();
            self.shutdown = None;
        } else {
            log::warn!(
                "worker already stopped, worker_id={}, stream_id={}",
                id.0,
                stream_id
            );
            return Err(WorkerError::NotFound);
        }
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "Result<WorkerId, WorkerError>")]
pub struct StartWorker {
    pub id: Option<WorkerId>,
    pub process: Process,
}

#[derive(Message)]
#[rtype(result = "Result<(), WorkerError>")]
pub struct StopWorker {
    pub id: WorkerId,
}

#[derive(Message)]
#[rtype(result = "Result<(), WorkerError>")]
pub struct RestartWorker {
    pub id: WorkerId,
}

#[derive(Message)]
#[rtype(result = "Option<WorkerState>")]
pub struct GetWorkerState {
    pub id: WorkerId,
}

#[derive(Message)]
#[rtype(result = "Vec<(WorkerId, WorkerState)>")]
pub struct ListWorkers;

#[derive(Message)]
#[rtype(result = "()")]
pub struct WorkerExited {
    pub id: WorkerId,
    pub success: bool,
    pub reason: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ShutdownWorkers;
