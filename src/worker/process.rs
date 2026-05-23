use crate::worker::error::{WorkerError, WorkerResult};
use crate::worker::filo::read_fifo_to_channel;
use bytes::Bytes;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::select;
use tokio::sync::{mpsc, oneshot, watch};

pub const OUTPUT_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct Process {
    pub program: String,
    pub args: Vec<String>,
    pub stdin: Option<String>,

    video_fifo: String,
    audio_fifo: String,
    video_tx: mpsc::Sender<Bytes>,
    audio_tx: mpsc::Sender<Bytes>,

    ffmpeg_ready_tx: watch::Sender<bool>,
    #[allow(dead_code)]
    publisher_ready_rx: watch::Receiver<bool>,
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
        video_tx: mpsc::Sender<Bytes>,
        audio_tx: mpsc::Sender<Bytes>,
        publisher_ready_rx: watch::Receiver<bool>,
        ffmpeg_ready_tx: watch::Sender<bool>,
    ) -> WorkerResult<Process> {
        let video_fifo = format!("/tmp/relay-{stream_id}-video.fmp4");
        let audio_fifo = format!("/tmp/relay-{stream_id}-audio.fmp4");

        crate::worker::filo::cleanup_fifo(&video_fifo)
            .map_err(|e| WorkerError::Filo(e.to_string()))?;
        crate::worker::filo::cleanup_fifo(&audio_fifo)
            .map_err(|e| WorkerError::Filo(e.to_string()))?;

        crate::worker::filo::create_fifo(&video_fifo)
            .map_err(|e| WorkerError::Filo(e.to_string()))?;
        crate::worker::filo::create_fifo(&audio_fifo)
            .map_err(|e| WorkerError::Filo(e.to_string()))?;

        let video_fifo_arg = video_fifo.clone();
        let audio_fifo_arg = audio_fifo.clone();
        Ok(Process {
            program: "ffmpeg".into(),
            args: vec![
                "-y".into(),
                "-hide_banner".into(),
                "-loglevel".into(),
                "info".into(),
                // --
                "-protocol_whitelist".into(),
                "file,pipe,udp,rtp".into(),
                // --
                // no buffer for low latency
                //"-fflags".into(),
                //"nobuffer".into(),
                "-flags".into(),
                "low_delay".into(),
                // timestamp clock settings
                "-fflags".into(),
                "+genpts".into(),
                "-use_wallclock_as_timestamps".into(),
                "1".into(),
                //--
                // probe size set for H264 RTP
                "-analyzeduration".into(),
                "10000000".into(),
                "-probesize".into(),
                "10000000".into(),
                // sdp Input
                "-f".into(),
                "sdp".into(),
                "-i".into(),
                "pipe:0".into(),
                // ---
                // VIDEO OUTPUT Video is already H.264 from WebRTC/RTP.
                "-map".into(),
                "0:v:0".into(),
                "-an".into(),
                // Video is already H.264 from WebRTC/RTP.
                "-c:v".into(),
                "copy".into(),
                "-avoid_negative_ts".into(), // timestamp settings
                "make_zero".into(),
                "-f".into(),
                "mp4".into(),
                "-movflags".into(),
                "frag_keyframe+empty_moov+default_base_moof+separate_moof+cmaf".into(),
                video_fifo_arg.into(),
                // ---
                // AUDIO OUTPUT
                "-map".into(),
                "0:a:0".into(),
                "-vn".into(),
                // Audio: WebRTC Opus -> AAC for MP4/CMAF.
                "-c:a".into(),
                "aac".into(),
                "-ar".into(),
                "48000".into(),
                "-ac".into(),
                "2".into(),
                "-b:a".into(),
                "128k".into(),
                // --
                // Low-latency fragmented MP4 / CMAF-like output.
                "-avoid_negative_ts".into(), // timestamp settings
                "make_zero".into(),
                // force audio pkg on every frame
                "-frag_duration".into(),
                "100000".into(),
                "-f".into(),
                "mp4".into(),
                "-movflags".into(),
                "frag_keyframe+empty_moov+default_base_moof+separate_moof+cmaf".into(),
                audio_fifo_arg.into(),
            ],
            stdin: Some(sdp.into()),
            video_tx,
            audio_tx,
            video_fifo: video_fifo.clone(),
            audio_fifo: audio_fifo.clone(),
            ffmpeg_ready_tx,
            publisher_ready_rx,
        })
    }

    pub async fn run(
        &mut self,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<String, WorkerError> {
        let mut cmd = Command::new(&self.program);

        cmd.args(&self.args)
            .stdout(Stdio::null())
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

        let video_reader = tokio::spawn(read_fifo_to_channel(
            "video",
            self.video_fifo.clone(),
            self.video_tx.clone(),
        ));

        let audio_reader = tokio::spawn(read_fifo_to_channel(
            "audio",
            self.audio_fifo.clone(),
            self.audio_tx.clone(),
        ));

        // wait for publisher ready
        // while !*self.publisher_ready_rx.borrow() {
        //     select! {
        //         _ = &mut shutdown_rx => {
        //             log::info!("shutdown while waiting for publisher ready");
        //             return Ok("stopped".to_string());
        //         }
        //
        //         changed = self.publisher_ready_rx.changed() => {
        //             if changed.is_err() {
        //                 return Err(WorkerError::ProcessFailed(
        //                     "publisher ready channel closed".to_string()
        //                 ));
        //             }
        //         }
        //     }
        // }

        // send ready signal
        let _ = self.ffmpeg_ready_tx.send(true).map_err(|e| {
            WorkerError::ProcessFailed(format!("send ffmpeg ready: {}", e.to_string()))
        })?;

        log::info!("ffmpeg Process Started:");

        // start ffmpeg
        loop {
            select! {
                biased;

                _ = &mut shutdown_rx => {
                    log::info!("ffmpeg AV shutdown requested");

                    let _ = child.kill().await;
                    let _ = child.wait().await;

                    video_reader.abort();
                    audio_reader.abort();

                    return Ok("stopped".to_string());
                }

                status = child.wait() => {
                    let status = status.map_err(|e| WorkerError::Spawn(e.to_string()))?;

                    video_reader.abort();
                    audio_reader.abort();

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
