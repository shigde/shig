use bytes::Bytes;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use crate::worker::error::{WorkerError, WorkerResult};
use crate::worker::process::OUTPUT_BUFFER_SIZE;

pub fn create_fifo(path: &str) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(std::path::Path::new(path).as_os_str().as_bytes())?;

    let result = unsafe {
        libc::mkfifo(c_path.as_ptr(), 0o600)
    };

    if result == 0 {
        Ok(())
    } else {
        let err = std::io::Error::last_os_error();

        if err.kind() == std::io::ErrorKind::AlreadyExists {
            Ok(())
        } else {
            Err(err)
        }
    }
}

pub async fn read_fifo_to_channel(
    label: &'static str,
    path: String,
    tx: mpsc::Sender<Bytes>,
) -> WorkerResult<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .read(true)
        .open(&path)
        .await
        .map_err(|e| WorkerError::Filo(format!("open {label} fifo: {e}")))?;

    let mut buf = vec![0u8; OUTPUT_BUFFER_SIZE];

    loop {
        match file.read(&mut buf).await {
            Ok(0) => {
                log::info!("{label} fifo closed");
                return Ok(());
            }

            Ok(n) => {
                let chunk = Bytes::copy_from_slice(&buf[..n]);

                if tx.send(chunk).await.is_err() {
                    log::info!("{label} receiver dropped");
                    return Ok(());
                }
            }

            Err(err) => {
                return Err(WorkerError::Filo(format!(
                    "read {label} fifo failed: {err}"
                )));
            }
        }
    }
}