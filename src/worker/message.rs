use crate::worker::error::WorkerError;
use crate::worker::process::Process;
use crate::worker::state::WorkerState;
use crate::worker::WorkerId;
use actix::Message;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct WorkerHandle {
    pub state: WorkerState,
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    pub process: Process,
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
