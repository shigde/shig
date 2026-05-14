use crate::sfu::relay::cmaf::cmaf_splitter::CmafItem;
use crate::sfu::relay::cmaf::prepared_cmaf_track::PreparedCmafTrack;
use crate::sfu::relay::error::{RelayError, RelayResult};
use bytes::Bytes;
use hang::moq_lite;

use tokio_util::sync::CancellationToken;

pub async fn write_cmaf_track(
    label: &'static str,
    mut track: moq_lite::TrackProducer,
    prepared: &mut PreparedCmafTrack,
    cancel: CancellationToken,
) -> RelayResult<()> {
    for fragment in prepared.pending.drain(..) {
        if cancel.is_cancelled() {
            log::info!("{} CMAF track cancelled", label);
            return Ok(());
        }

        write_fragment(&mut track, fragment)?;
    }

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                log::info!("{} CMAF track cancelled", label);
                return Ok(());
            }

            maybe_chunk = prepared.rx.recv() => {
                let Some(chunk) = maybe_chunk else {
                    log::info!("{} CMAF track finished", label);
                    return Ok(());
                };

                let items = prepared.splitter.push(chunk)?;

                for item in items {
                    if cancel.is_cancelled() {
                        log::info!("{} CMAF track cancelled", label);
                        return Ok(());
                    }

                    match item {
                        CmafItem::Init(_) => {
                            // Init is already in the catalog.
                        }

                        CmafItem::Fragment(fragment) => {
                            write_fragment(&mut track, fragment)?;
                        }
                    }
                }
            }
        }
    }
}

pub fn write_fragment(track: &mut moq_lite::TrackProducer, fragment: Bytes) -> RelayResult<()> {
    let mut group = track
        .append_group()
        .map_err(|e| RelayError::CmafWrite(e.to_string()))?;

    group
        .write_frame(fragment)
        .map_err(|e| RelayError::CmafWrite(e.to_string()))?;

    group
        .finish()
        .map_err(|e| RelayError::CmafWrite(e.to_string()))?;

    Ok(())
}
