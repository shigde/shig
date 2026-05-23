use std::os::unix::fs::FileTypeExt;
use bytes::Bytes;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use crate::worker::error::{WorkerError, WorkerResult};
use crate::worker::process::OUTPUT_BUFFER_SIZE;


pub fn cleanup_fifo(path: &str) -> std::io::Result<()> {
    let p = std::path::Path::new(path);

    if p.exists() {
        std::fs::remove_file(p)?;
    }

    Ok(())
}

pub fn create_fifo(path: &str) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::fs;

    let p = std::path::Path::new(path);

    // Wenn existiert → prüfen & ggf. löschen
    if p.exists() {
        let metadata = fs::metadata(p)?;

        if metadata.file_type().is_fifo() {
            fs::remove_file(p)?;
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Path exists but is not a FIFO: {}", path),
            ));
        }
    }

    let c_path = CString::new(p.as_os_str().as_bytes())?;

    let result = unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) };

    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
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