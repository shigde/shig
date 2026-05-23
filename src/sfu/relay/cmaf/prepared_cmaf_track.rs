use crate::sfu::relay::cmaf::cmaf_splitter::{CmafItem, CmafSplitter};
use crate::sfu::relay::error::{RelayError, RelayResult};
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub struct PreparedCmafTrack {
    #[allow(dead_code)]
    pub(crate) name: String,
    pub(crate) init: Bytes,
    pub(crate) pending: Vec<Bytes>,
    pub(crate) splitter: CmafSplitter,
    pub(crate) rx: mpsc::Receiver<Bytes>,
}

impl PreparedCmafTrack {
    pub async fn build(
        name: String,
        mut rx: mpsc::Receiver<Bytes>,
        cancel: CancellationToken,
    ) -> RelayResult<PreparedCmafTrack> {
        let mut splitter = CmafSplitter::default();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    return Err(RelayError::CmafPreparation("CMAF preparation cancelled".to_string()));
                }

                maybe_chunk = rx.recv() => {
                    let Some(chunk) = maybe_chunk else {
                        return Err(RelayError::CmafPreparation("CMAF stream ended before init segment".to_string()));
                    };

                    let items = splitter.push(chunk)?;

                    let mut init = None;
                    let mut pending = Vec::new();

                    for item in items {
                        match item {
                            CmafItem::Init(bytes) => init = Some(bytes),
                            CmafItem::Fragment(bytes) => pending.push(bytes),
                        }
                    }

                    if let Some(init) = init {
                        log::info!("CMAF init segment received for track {}", name);
                        return Ok(PreparedCmafTrack {
                            name,
                            init,
                            pending,
                            splitter,
                            rx,
                        });
                    }
                }
            }
        }
    }
}
