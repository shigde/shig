use bytes::{Bytes, BytesMut};
use moq_mux::container::fmp4;
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use crate::sfu::relay::error::{RelayError, RelayResult};
use crate::util::stop_guard::StopGuard;

pub struct HangPublisher {
    origin: moq_net::OriginProducer,
    name: String,
    pkg_rx: mpsc::Receiver<Bytes>,
    #[allow(dead_code)]
    publisher_ready_tx: watch::Sender<bool>,
}

impl HangPublisher {
    pub fn new(
        origin: moq_net::OriginProducer,
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

    pub async fn run(mut self, cancel: CancellationToken, stopped: CancellationToken) -> RelayResult<()> {
        log::info!("hang publisher started");
        let _guard = StopGuard(stopped);

        let mut broadcast = moq_net::Broadcast::new().produce();
        let catalog = moq_mux::catalog::hang::Producer::new(&mut broadcast)
            .map_err(|e| RelayError::PublisherError(e.to_string()))?;


        let mut import = fmp4::Import::new(
            broadcast.clone(),
            catalog.clone(),
        );

        self.origin.publish_broadcast(
            self.name.clone(),
            broadcast.consume(),
        );


        let mut buffer = BytesMut::new();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    break;
                }

                Some(bytes) = self.pkg_rx.recv() => {
                    log::info!("received fmp4 bytes: {}", bytes.len());

                    buffer.extend_from_slice(&bytes);

                    match import.decode(&mut buffer) {
                        Ok(_) => {
                            log::info!("decoded fmp4 buffer, remaining={}", buffer.len());
                        }
                        Err(e) => {
                            log::error!("fmp4 import error: {:?}", e);
                            return Err(RelayError::PublisherError(e.to_string()));
                        }
                    }
                }
            }
        }

        import.finish().map_err(|e| RelayError::PublisherError(e.to_string()))?;
        Ok(())
    }
}
