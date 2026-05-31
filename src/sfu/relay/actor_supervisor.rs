use crate::sfu::relay::error::{RelayError, RelayResult};
use crate::worker::manager::WorkerManager;
use crate::worker::message::StopWorker;
use crate::worker::WorkerId;
use actix::Addr;
use tokio_util::sync::CancellationToken;

// Actor ------------------------
// -[channel,rtp]-> rtp-forwarder
//   -[udp,rtp]-> ffmpeg-process
//     -[filo,mp4]-> read_fifo_to_channel
//      -[channel,mp4]-> PreparedTrack
//       -[channel,mp4]-> cmaf-track-writer
//         -[fragment, CMAF]-> relay:publisher
#[derive(Clone)]
pub struct RelayActorSupervisor {
    stream_id: String,
    worker_manager: Addr<WorkerManager>,
    pub shutdown: CancellationToken,
    pub is_down: CancellationToken,

    pub publisher: CancellationToken,
    pub publisher_stopped: CancellationToken,

    pub rest: CancellationToken,
    pub process_stopped: CancellationToken,
    pub forwarder: CancellationToken,
}

impl RelayActorSupervisor {
    pub fn new(stream_id: String, worker_manager: Addr<WorkerManager>) -> Self {
        let shutdown = CancellationToken::new();

        let publisher = CancellationToken::new();
        let publisher_stopped = CancellationToken::new();

        let rest = CancellationToken::new();
        let process_stopped = CancellationToken::new();
        let forwarder = rest.child_token();

        Self {
            stream_id,
            worker_manager,
            shutdown,
            is_down: CancellationToken::new(),
            publisher,
            publisher_stopped,
            rest,
            process_stopped,
            forwarder,
        }
    }

    pub async fn start(&self, worker_id: WorkerId) {
        tokio::select! {
        _ = self.shutdown.cancelled() => {
            log::info!("shutdown requested, stream_id= {}, worker_id={}", self.stream_id, worker_id.0);
        }

        _ = self.monitor_subtasks() => {
            log::warn!("subtask stopped, triggering shutdown");
            self.shutdown.cancel();
        }
    }

        self.shutdown_sequence(worker_id).await;
    }

    async fn shutdown_sequence(&self, worker_id: WorkerId) {
        let _guard = DownGuard(self.is_down.clone());

        log::info!("shutting down relay actor: stream_id= {}, worker_id={}", self.stream_id, worker_id.0);
        self.shutdown.cancelled().await;

        self.publisher.cancel();
        log::info!("stopping publisher: stream_id= {}, worker_id={}", self.stream_id, worker_id.0);

        log::info!("waiting for publisher to stop: stream_id= {}, worker_id={}", self.stream_id, worker_id.0);
        self.publisher_stopped.cancelled().await;
        log::info!("publisher stopped: stream_id= {}, worker_id={}", self.stream_id, worker_id.0);

        log::info!("send signal to stop worker process: stream_id= {}, worker_id={}", self.stream_id, worker_id.0);
        if let Err(e) = self.stop_worker(worker_id.clone()).await {
            log::error!(
                "Failed to stop worker, worker_is= {}, error={:?}",
                worker_id.0,
                e
            );
            self.process_stopped.cancel();
        }

        log::info!("waiting for worker process to stop: stream_id= {}, worker_id={}", self.stream_id, worker_id.0);
        self.process_stopped.cancelled().await;

        log::info!("stopping forwarder: stream_id= {}, worker_id={}", self.stream_id, worker_id.0);
        self.rest.cancel();

        log::info!("ffmpeg process stopped: stream_id= {}, worker_id={}", self.stream_id, worker_id.0);
    }

    async fn monitor_subtasks(&self) {
        tokio::select! {
            _ = self.publisher.cancelled() => {
                self.shutdown.cancel();
            }

            _ = self.process_stopped.cancelled() => {
                self.shutdown.cancel();
            }

            _ = self.forwarder.cancelled() => {
                self.shutdown.cancel();
            }

            _ = self.rest.cancelled() => {
                self.shutdown.cancel();
            }
        }
    }

    async fn stop_worker(&self, id: WorkerId) -> RelayResult<()> {
        self.worker_manager
            .send(StopWorker { id })
            .await
            .map_err(|e| RelayError::WorkerMailboxError(e.to_string()))?
            .map_err(|e| RelayError::WorkerError(e))?;

        Ok(())
    }
}

struct DownGuard(CancellationToken);

impl Drop for DownGuard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
