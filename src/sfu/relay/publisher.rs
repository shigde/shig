use crate::sfu::relay::error::{RelayError, RelayResult};
use crate::util::stop_guard::StopGuard;
use bytes::Bytes;
use moq_mux::container::fmp4;
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

pub struct HangPublisher {
    origin: moq_net::origin::Producer,
    name: String,
    pkg_rx: mpsc::Receiver<Bytes>,
    #[allow(dead_code)]
    publisher_ready_tx: watch::Sender<bool>,
}

impl HangPublisher {
    pub fn new(
        origin: moq_net::origin::Producer,
        name: String,
        pkg_rx: mpsc::Receiver<Bytes>,
        publisher_ready_tx: watch::Sender<bool>,
    ) -> Self {
        Self {
            origin,
            name,
            pkg_rx,
            publisher_ready_tx,
        }
    }

    pub async fn run(
        mut self,
        cancel: CancellationToken,
        stopped: CancellationToken,
    ) -> RelayResult<()> {
        log::info!("hang publisher started");
        let _guard = StopGuard(stopped);

        let mut broadcast = self
            .origin
            .publish(&self.name, Default::default())
            .map_err(|e| RelayError::PublisherError(e.to_string()))?;
        let catalog = moq_mux::catalog::Producer::<()>::new(&mut broadcast, Default::default())
            .map_err(|e| RelayError::PublisherError(e.to_string()))?;

        let mut import = fmp4::Import::new(broadcast.clone(), catalog.reserve());

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    break;
                }

                Some(bytes) = self.pkg_rx.recv() => {
                    log::trace!("received fmp4 bytes: {}", bytes.len());

                    match import.decode(&bytes) {
                        Ok(_) => {
                            log::trace!("decoded fmp4 input");
                        }
                        Err(e) => {
                            log::error!("fmp4 import error: {:?}", e);
                            return Err(RelayError::PublisherError(e.to_string()));
                        }
                    }
                }
            }
        }

        import
            .finish()
            .map_err(|e| RelayError::PublisherError(e.to_string()))?;
        Ok(())
    }
}
