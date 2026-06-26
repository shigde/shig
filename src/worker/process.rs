use crate::worker::error::{WorkerError, WorkerResult};
use bytes::{Bytes, BytesMut};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::select;
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

pub const OUTPUT_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct Process {
    pub stream_id: String,
    pub program: String,
    pub args: Vec<String>,
    pub stdin: Option<String>,

    pkg_tx: mpsc::Sender<Bytes>,

    ffmpeg_ready_tx: watch::Sender<bool>,
    stopped: CancellationToken,
}

impl Process {
    pub fn build_ffmpeg_sdp(
        video_port: u16,
        audio_port: u16,
        video_pt: u8,
        audio_pt: u8,
        video_sdp_fmtp_line: String,
    ) -> String {
        format!(
            "v=0\r\n\
o=- 0 0 IN IP4 127.0.0.1\r\n\
s=Shig Stream\r\n\
c=IN IP4 127.0.0.1\r\n\
t=0 0\r\n\
m=audio {audio_port} RTP/AVP {audio_pt}\r\n\
a=rtpmap:{audio_pt} opus/48000/2\r\n\
a=fmtp:{audio_pt} minptime=10;useinbandfec=1\r\n\
m=video {video_port} RTP/AVP {video_pt}\r\n\
a=rtpmap:{video_pt} H264/90000\r\n\
a=fmtp:{video_pt} {video_sdp_fmtp_line}\r\n"
        )
    }

    pub fn build(
        sdp: &str,
        stream_id: &str,
        pkg_tx: mpsc::Sender<Bytes>,
        ffmpeg_ready_tx: watch::Sender<bool>,
        stopped: CancellationToken,
    ) -> WorkerResult<Process> {
        Ok(Process {
            stream_id: stream_id.to_string(),
            program: "ffmpeg".into(),
            args: vec![
                "-hide_banner".into(),
                "-loglevel".into(),
                "info".into(),

                "-protocol_whitelist".into(),
                "file,pipe,udp,rtp".into(),

                "-fflags".into(),
                "+genpts+igndts".into(),

                "-flags".into(),
                "low_delay".into(),

                "-analyzeduration".into(),
                "10000000".into(),

                "-probesize".into(),
                "10000000".into(),

                "-f".into(),
                "sdp".into(),

                "-i".into(),
                "pipe:0".into(),

                "-map".into(),
                "0:v:0".into(),

                "-map".into(),
                "0:a:0".into(),

                "-c:v".into(),
                "copy".into(),

                "-c:a".into(),
                "aac".into(),

                "-ar".into(),
                "48000".into(),

                "-ac".into(),
                "2".into(),

                "-b:a".into(),
                "128k".into(),

                "-af".into(),
                "aresample=async=1:first_pts=0".into(),

                "-map_metadata".into(),
                "-1".into(),

                "-frag_duration".into(),
                "100000".into(), // 100ms

                "-f".into(),
                "mp4".into(),
                
                "-movflags".into(),
                "frag_keyframe+empty_moov+delay_moov+default_base_moof+separate_moof+cmaf".into(),

                "pipe:1".into(),
            ],
            stdin: Some(sdp.into()),
            pkg_tx,
            ffmpeg_ready_tx,
            stopped,
        })
    }

    pub async fn run(&mut self, cancel_token: CancellationToken) -> Result<String, WorkerError> {
        let mut cmd = Command::new(&self.program);
        cmd.kill_on_drop(true);

        cmd.args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(if self.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            });

        let mut child = cmd.spawn().map_err(|e| WorkerError::Spawn(e.to_string()))?;

        if let Some(input) = self.stdin.take() {
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(input.as_bytes())
                    .await
                    .map_err(|e| WorkerError::Spawn(e.to_string()))?;

                let _ = stdin.shutdown().await;
            }
        }

        let mut stderr_lines = child.stderr.take().map(|stderr| {
            let reader = BufReader::new(stderr);
            reader.lines()
        });


        // send ready signal
        let _ = self.ffmpeg_ready_tx.send(true).map_err(|e| {
            WorkerError::ProcessFailed(format!("send ffmpeg ready: {}", e.to_string()))
        })?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| WorkerError::ProcessFailed("missing ffmpeg stdout".into()))?;

        let mut buffer = BytesMut::with_capacity(OUTPUT_BUFFER_SIZE);

        let stream_id = self.stream_id.clone();
        log::info!("ffmpeg Process Started: stream_id={}", stream_id);

        // start ffmpeg
        let canceled = cancel_token.clone();
        let stopped = self.stopped.clone();
        loop {
            select! {
                biased;

                _ = canceled.cancelled() => {
                    log::info!("ffmpeg AV shutdown requested: stream_id={}", stream_id);

                    let _ = child.kill().await;
                    let _ = child.wait().await;

                    stopped.cancel();
                    return Ok("stopped".to_string());
                }

                read = stdout.read_buf(&mut buffer) => {
                    let n = read.map_err(|e| WorkerError::ProcessFailed(e.to_string()))?;

                    if n == 0 {
                        return Ok("stopped".to_string());
                    }

                    let bytes = buffer.split().freeze();

                    self.pkg_tx
                        .send(bytes)
                        .await
                        .map_err(|e| WorkerError::ProcessFailed(format!("send ffmpeg pkg: {}", e)))?;
                }

                status = child.wait() => {
                    let status = status.map_err(|e| WorkerError::Spawn(e.to_string()))?;

                    stopped.cancel();

                    return if status.success() {
                        Ok(format!("exit code {}", status.code().unwrap_or(0)))
                    } else {
                        Err(WorkerError::ProcessFailed(format!(
                            "exit code {}",
                            status.code().unwrap_or(-1)
                        )))
                    };
                }

                line = async {
                    match stderr_lines.as_mut() {
                        Some(lines) => lines.next_line().await,
                        None => Ok(None),
                    }
                }, if stderr_lines.is_some() => {
                    match line {
                        Ok(Some(line)) => log::info!("[ffmpeg] {}", line),
                        Ok(None) => stderr_lines = None,
                        Err(err) => {
                            log::error!("failed to read ffmpeg stderr: {}", err);
                            stderr_lines = None;
                        }
                    }
                }
            }
        }
    }
}
