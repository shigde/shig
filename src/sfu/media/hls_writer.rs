//! Persist moq-hls output. Subscription, timing and transmuxing belong to moq-hls.
use anyhow::{bail, ensure, Context};
use moq_hls::export::{master, renditions::Event, Broadcaster, Kind, Rendition};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{fs, task::JoinSet};
use tokio_util::sync::CancellationToken;

pub(super) struct HlsWriter {
    broadcaster: Arc<Broadcaster>,
    directory: PathBuf,
    after: u64,
}

struct SavedRendition {
    rendition: Arc<Rendition>,
    uri: String,
    duration: Duration,
}

impl HlsWriter {
    pub async fn open(source: moq_mux::Source, directory: PathBuf) -> anyhow::Result<Self> {
        let (broadcaster, after) = Self::prepare(source).await?;
        // Only an actual recording creates a directory; never create the configured root here.
        fs::create_dir(&directory)
            .await
            .context("Cannot create recording directory")?;
        Ok(Self {
            broadcaster,
            directory,
            after,
        })
    }

    pub(super) async fn prepare(
        source: moq_mux::Source,
    ) -> anyhow::Result<(Arc<Broadcaster>, u64)> {
        // Retain timeline metadata for the entire recording, rather than the live 16s window.
        // Media bytes are still fetched and written one segment at a time.
        let mut config = moq_hls::export::Config::default();
        config.window = Duration::MAX;
        let broadcaster =
            tokio::time::timeout(Duration::from_secs(10), Broadcaster::new(source, config))
                .await
                .context("Timed out resolving recording source")??;
        tokio::time::timeout(Duration::from_secs(10), broadcaster.ready())
            .await
            .context("Timed out waiting for HLS media")?;
        ensure!(!broadcaster.is_closed(), "Recording source is closed");
        // Segment numbers are shared by every rendition. Establish one common start
        // boundary before returning, including when an inline codec must build its init.
        let mut renditions = broadcaster.renditions();
        let after = tokio::time::timeout(Duration::from_secs(15), async {
            while let Some(event) = renditions.next().await {
                let Event::Added(rendition) = event else {
                    continue;
                };
                if !matches!(rendition.kind, Kind::Video) {
                    continue;
                }
                // An advertised rendition is not necessarily initialized yet. The cursor
                // waits for actual media and bootstraps the inline H.264 parameter sets.
                let mut segments = rendition.segments();
                while segments.next().await?.is_some() {
                    if let Some(playlist) = rendition.playlist(None).await? {
                        if let Some(after) = last_segment(&playlist) {
                            return Ok(after);
                        }
                    }
                }
            }
            bail!("Recording source ended before HLS initialization became available")
        })
        .await
        .context("Timed out waiting for HLS recording readiness")??;
        Ok((broadcaster, after))
    }

    pub async fn run(self, stop: CancellationToken) -> anyhow::Result<u64> {
        let result = self.record(stop.clone()).await;
        stop.cancel();
        result
    }

    async fn record(&self, stop: CancellationToken) -> anyhow::Result<u64> {
        let mut events = self.broadcaster.renditions();
        let mut workers = JoinSet::new();
        let mut saved = Vec::new();
        let mut index = 0;
        loop {
            tokio::select! {
                biased;
                _ = stop.cancelled() => break,
                result = workers.join_next(), if !workers.is_empty() => {
                    saved.push(result.context("Missing HLS worker")???);
                    // The media timeline can end before the catalog subscription closes.
                    if workers.is_empty() { break; }
                }
                event = events.next() => match event {
                    Some(Event::Added(rendition)) => {
                        let relative = format!("track-{index}");
                        index += 1;
                        let directory = self.directory.join(&relative);
                        let stop = stop.clone();
                        // Do not include already completed live segments from before Record.
                        let after = self.after;
                        workers.spawn(async move {
                            record_rendition(rendition, directory, relative, after, stop).await
                        });
                    }
                    Some(Event::Removed { .. }) => {
                        bail!("Source rendition changed during recording");
                    }
                    None => break,
                    _ => {}
                }
            }
        }
        // On a natural source end, drain the library's remaining completed segments.
        // On an explicit stop, workers finish any disk write already in progress.
        while let Some(result) = workers.join_next().await {
            saved.push(result??);
        }
        ensure!(!saved.is_empty(), "No HLS media was recorded");
        let mut video = Vec::new();
        let mut audio = Vec::new();
        let mut duration = Duration::ZERO;
        for saved in saved {
            duration = duration.max(saved.duration);
            let rendition = saved.rendition;
            match rendition.kind {
                Kind::Video => video.push(master::VideoVariant {
                    uri: saved.uri,
                    bandwidth: rendition.bandwidth(),
                    width: rendition.width,
                    height: rendition.height,
                    codec: rendition.codec.clone(),
                }),
                Kind::Audio => audio.push(master::AudioVariant {
                    uri: saved.uri,
                    name: rendition.name.clone(),
                    bandwidth: rendition.bandwidth(),
                    codec: rendition.codec.clone(),
                }),
            }
        }
        write_playlist(
            &self.directory.join("master.m3u8"),
            &master::render(&video, &audio),
        )
        .await?;
        Ok(u64::try_from(duration.as_millis())?)
    }
}

async fn record_rendition(
    rendition: Arc<Rendition>,
    directory: PathBuf,
    relative: String,
    after: u64,
    stop: CancellationToken,
) -> anyhow::Result<SavedRendition> {
    let mut cursor = rendition.segments();
    let mut saved = BTreeSet::new();
    let mut duration = Duration::ZERO;
    let mut init_name = None;
    fs::create_dir(&directory).await?;
    fs::create_dir(directory.join("seg")).await?;
    loop {
        let segment = tokio::select! {
            biased;
            _ = stop.cancelled() => break,
            segment = cursor.next() => segment?,
        };
        let Some(segment) = segment else { break };
        if segment.segment <= after {
            continue;
        }
        let playlist = rendition
            .media_playlist(None)
            .context("Missing HLS playlist")?;
        let name = playlist
            .lines()
            .find_map(|line| line.strip_prefix("#EXT-X-MAP:URI=\"")?.strip_suffix('"'))
            .context("Missing HLS initialization URI")?;
        ensure!(
            !name.contains('/') && name.starts_with("init.") && name.ends_with(".mp4"),
            "Invalid HLS initialization URI"
        );
        if let Some(previous) = &init_name {
            ensure!(
                previous == name,
                "Source initialization changed during recording"
            );
        } else {
            let init = cursor
                .init()
                .await?
                .context("Missing HLS initialization segment")?;
            fs::write(directory.join(name), init).await?;
            init_name = Some(name.to_owned());
        }
        fs::write(
            directory
                .join("seg")
                .join(format!("{}.m4s", segment.segment)),
            segment.media,
        )
        .await?;
        saved.insert(segment.segment);
        duration += segment.duration;
    }
    let playlist = rendition
        .media_playlist(None)
        .context("Missing final HLS playlist")?;
    let playlist = finish_playlist(&playlist, &saved)?;
    write_playlist(&directory.join("media.m3u8"), &playlist).await?;
    Ok(SavedRendition {
        rendition,
        uri: format!("{relative}/media.m3u8"),
        duration,
    })
}

fn segment_number(uri: &str) -> Option<u64> {
    uri.strip_prefix("seg/")?.strip_suffix(".m4s")?.parse().ok()
}

fn last_segment(playlist: &str) -> Option<u64> {
    playlist.lines().filter_map(segment_number).max()
}

/// Keep moq-hls's playlist syntax, timing, gaps and discontinuities. Trim to the
/// files actually captured and end this recording without ending the live source.
fn finish_playlist(playlist: &str, saved: &BTreeSet<u64>) -> anyhow::Result<String> {
    let first = *saved
        .first()
        .context("No complete HLS segment was recorded")?;
    let last = *saved.last().unwrap();
    let mut out = String::new();
    let mut block = String::new();
    let mut listed = BTreeSet::new();
    for line in playlist.lines() {
        if let Some(number) = segment_number(line) {
            if (first..=last).contains(&number) {
                ensure!(
                    saved.contains(&number) || block.lines().any(|l| l == "#EXT-X-GAP"),
                    "HLS segment {number} was lost before it could be recorded"
                );
                out.push_str(&block);
                out.push_str(line);
                out.push('\n');
                if saved.contains(&number) {
                    listed.insert(number);
                }
            }
            block.clear();
        } else if line.starts_with("#EXTINF:")
            || line == "#EXT-X-GAP"
            || line == "#EXT-X-DISCONTINUITY"
        {
            block.push_str(line);
            block.push('\n');
        } else if line.starts_with("#EXT-X-MEDIA-SEQUENCE:") {
            out.push_str(&format!("#EXT-X-MEDIA-SEQUENCE:{first}\n"));
        } else if line != "#EXT-X-ENDLIST"
            && !line.starts_with("#EXT-X-PLAYLIST-TYPE:")
            && !line.starts_with("#EXT-X-PROGRAM-DATE-TIME:")
        {
            out.push_str(line);
            out.push('\n');
        }
    }
    ensure!(
        &listed == saved,
        "Recorded segments are missing from the HLS timeline"
    );
    out.push_str("#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-ENDLIST\n");
    Ok(out)
}

async fn write_playlist(path: &Path, contents: &str) -> anyhow::Result<()> {
    let temporary = path.with_extension("m3u8.tmp");
    fs::write(&temporary, contents).await?;
    fs::rename(temporary, path).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const PLAYLIST: &str = "#EXTM3U\n#EXT-X-VERSION:6\n#EXT-X-TARGETDURATION:2\n#EXT-X-MEDIA-SEQUENCE:10\n#EXT-X-MAP:URI=\"init.abc.mp4\"\n#EXTINF:2.00000,\nseg/10.m4s\n#EXTINF:2.00000,\nseg/11.m4s\n#EXT-X-DISCONTINUITY\n#EXTINF:2.00000,\nseg/12.m4s\n#EXTINF:2.00000,\nseg/13.m4s\n";

    #[test]
    fn recording_playlist_only_lists_saved_span_and_ends_independently() {
        let result = finish_playlist(PLAYLIST, &[11, 12].into()).unwrap();
        assert!(result.contains("#EXT-X-MEDIA-SEQUENCE:11\n"));
        assert!(result.contains("#EXT-X-MAP:URI=\"init.abc.mp4\""));
        assert!(!result.contains("seg/10.m4s"));
        assert!(!result.contains("seg/13.m4s"));
        assert!(result.contains("#EXT-X-DISCONTINUITY\n#EXTINF:2.00000,\nseg/12.m4s"));
        assert!(result.ends_with("#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-ENDLIST\n"));
    }

    #[test]
    fn missing_media_cannot_be_advertised_as_a_ready_recording() {
        assert!(finish_playlist(PLAYLIST, &[10, 12].into()).is_err());
        assert!(finish_playlist(PLAYLIST, &BTreeSet::new()).is_err());
    }

    #[tokio::test]
    async fn waits_for_initial_video_media_without_creating_a_recording_directory() {
        let origin = moq_tokio::origin::spawn();
        let mut broadcast = origin
            .publish("live/preparing", Default::default())
            .unwrap();
        let catalog =
            moq_mux::catalog::Producer::<()>::new(&mut broadcast, Default::default()).unwrap();
        let mut track = moq_mux::import::Track::video(
            broadcast.reserve_track("video").unwrap(),
            catalog.reserve(),
            moq_mux::import::VideoInit::new(moq_mux::import::VideoFormat::Vp8, bytes::Bytes::new()),
        )
        .unwrap();
        let source = moq_mux::Source::new(origin.consume(), "live/preparing");
        let mut preparing = tokio::spawn(HlsWriter::prepare(source));
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut preparing)
                .await
                .is_err()
        );
        for second in 0..6 {
            track
                .decode(
                    bytes::Bytes::from_static(&[
                        0x10, 0, 0, 0x9d, 0x01, 0x2a, 0x40, 0x01, 0xf0, 0x00,
                    ]),
                    Some(moq_net::Timestamp::from_micros(second * 1_000_000).unwrap()),
                )
                .unwrap();
            tokio::task::yield_now().await;
        }
        let (broadcaster, _) = tokio::time::timeout(Duration::from_secs(5), preparing)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(!broadcaster.is_closed());
        assert!(!broadcast.consume().is_closed());
    }

    #[tokio::test]
    async fn writes_library_segments_and_stops_without_closing_the_live_broadcast() {
        let root = std::env::temp_dir().join(format!("shig-hls-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).await.unwrap();
        let origin = moq_tokio::origin::spawn();
        let mut broadcast = origin.publish("live/test", Default::default()).unwrap();
        let mut catalog =
            moq_mux::catalog::Producer::<()>::new(&mut broadcast, Default::default()).unwrap();
        let mut track = moq_mux::import::Track::video(
            broadcast.reserve_track("video").unwrap(),
            catalog.reserve(),
            moq_mux::import::VideoInit::new(moq_mux::import::VideoFormat::Vp8, bytes::Bytes::new()),
        )
        .unwrap();
        let push = |track: &mut moq_mux::import::Track, seconds: u64| {
            track
                .decode(
                    bytes::Bytes::from_static(&[
                        0x10, 0, 0, 0x9d, 0x01, 0x2a, 0x40, 0x01, 0xf0, 0x00,
                    ]),
                    Some(moq_net::Timestamp::from_micros(seconds * 1_000_000).unwrap()),
                )
                .unwrap();
        };
        for second in 0..6 {
            push(&mut track, second);
        }
        let writer = HlsWriter::open(
            moq_mux::Source::new(origin.consume(), "live/test"),
            root.join("first"),
        )
        .await
        .unwrap();
        let stop = CancellationToken::new();
        let task = tokio::spawn(writer.run(stop.clone()));
        for second in 6..14 {
            push(&mut track, second);
            tokio::task::yield_now().await;
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(mut entries) = fs::read_dir(root.join("first/track-0/seg")).await {
                    if entries.next_entry().await.unwrap().is_some() {
                        break;
                    }
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        stop.cancel();
        let duration = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(duration > 0);
        assert!(
            !broadcast.consume().is_closed(),
            "stopping a recording must not stop live viewing"
        );
        let master = fs::read_to_string(root.join("first/master.m3u8"))
            .await
            .unwrap();
        assert!(master.contains("track-0/media.m3u8"));
        let playlist = fs::read_to_string(root.join("first/track-0/media.m3u8"))
            .await
            .unwrap();
        assert!(playlist.ends_with("#EXT-X-ENDLIST\n"));
        for uri in playlist
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
        {
            let media = fs::read(root.join("first/track-0").join(uri))
                .await
                .unwrap();
            assert_eq!(&media[4..8], b"moof");
        }
        // Another recording uses the same live source and completes on source shutdown.
        let second = HlsWriter::open(
            moq_mux::Source::new(origin.consume(), "live/test"),
            root.join("second"),
        )
        .await
        .unwrap();
        let second = tokio::spawn(second.run(CancellationToken::new()));
        for second in 14..22 {
            push(&mut track, second);
            tokio::task::yield_now().await;
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(mut entries) = fs::read_dir(root.join("second/track-0/seg")).await {
                    if entries.next_entry().await.unwrap().is_some() {
                        break;
                    }
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        track.finish().unwrap();
        catalog.finish().unwrap();
        broadcast.close();
        assert!(
            tokio::time::timeout(Duration::from_secs(5), second)
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                > 0
        );
        assert!(fs::read_to_string(root.join("second/track-0/media.m3u8"))
            .await
            .unwrap()
            .ends_with("#EXT-X-ENDLIST\n"));
        fs::remove_dir_all(root).await.unwrap();
    }
}
