use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::process::Stdio;
use tokio::select;
use crate::worker::error::WorkerError;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct Process {
    pub program: String,
    pub args: Vec<String>,
}

// Process {
// program: "ffmpeg".into(),
// args: vec![
//     "-i".into(),
//     "input.sdp".into(),
//     "-c:v".into(),
//     "copy".into(),
//     "-c:a".into(),
//     "aac".into(),
//     "out.mpd".into(),
// ],
// }


pub async fn run_process(
    process: Process,
    mut shutdown_rx: oneshot::Receiver<()>,
) -> Result<String, WorkerError> {
    let mut cmd = Command::new(&process.program);

    cmd.args(&process.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let mut child = cmd.spawn()
        .map_err(|e| WorkerError::Spawn(e.to_string()))?;
    
    if let Some(stderr) = child.stderr.take() {
        actix_rt::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                log::info!("[ffmpeg] {}", line);
            }
        });
    }
    
    select! {
        status = child.wait() => {
            let status = status.map_err(|e| WorkerError::Spawn(e.to_string()))?;

            if status.success() {
                Ok(format!("exit code {}", status.code().unwrap_or(0)))
            } else {
                Err(WorkerError::ProcessFailed(format!(
                    "exit code {}",
                    status.code().unwrap_or(-1)
                )))
            }
        }

        _ = &mut shutdown_rx => {
            log::info!("shutdown requested");

            let _ = child.kill().await;
            let _ = child.wait().await;

            Ok("stopped".to_string())
        }
    }
}