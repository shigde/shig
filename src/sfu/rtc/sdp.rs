use crate::sfu::media::track_info::{InboundTrackInfo, OutboundTrackInfo};
use crate::sfu::media::{MediaPurpose, RtpCodecType};
use webrtc::peer_connection::RTCSessionDescription;

pub(crate) fn parse_offered_track_info(sdp: &str) -> anyhow::Result<Vec<InboundTrackInfo>> {
    let mut result = Vec::new();
    let mut current_track_info: Option<InboundTrackInfo> = None;
    let mut current_has_description = false;

    for line in sdp.lines() {
        let line = line.trim();
        if line.starts_with("m=") {
            push_track_info(
                &mut result,
                current_track_info.take(),
                current_has_description,
            );
            current_has_description = false;
            current_track_info = if line.starts_with("m=audio") {
                Some(InboundTrackInfo::new(RtpCodecType::Audio))
            } else if line.starts_with("m=video") {
                Some(InboundTrackInfo::new(RtpCodecType::Video))
            } else {
                None
            };
        } else if line.starts_with("a=mid:") {
            if let Some(track_info) = current_track_info.as_mut() {
                track_info.mid = line.trim_start_matches("a=mid:").trim().to_string();
            }
        } else if line.starts_with("a=msid:") {
            if let Some(track_info) = current_track_info.as_mut() {
                let mut ids = line.trim_start_matches("a=msid:").split_whitespace();
                track_info.source_stream_id = ids.next().unwrap_or_default().to_string();
                track_info.source_track_id = ids.next().unwrap_or_default().to_string();
            }
        } else if line.starts_with("a=extmap:") {
            if let Some(track_info) = current_track_info.as_mut() {
                let value = line.trim_start_matches("a=extmap:");
                let mut parts = value.split_whitespace();
                let Some(id) = parts
                    .next()
                    .and_then(|value| value.split('/').next())
                    .and_then(|value| value.parse::<u8>().ok())
                else {
                    continue;
                };
                let Some(uri) = parts.next() else {
                    continue;
                };
                track_info.header_extensions.insert(id, uri.to_string());
            }
        } else if line.starts_with("i=") {
            if let Some(track_info) = current_track_info.as_mut() {
                current_has_description = true;
                let desc = line.trim_start_matches("i=").trim();
                let mut parts = desc.splitn(3, ' ');

                track_info.purpose = MediaPurpose::from_option_str(parts.next());
                track_info.muted = parse_muted(parts.next());
                track_info.info = parts.next().map(|s| s.to_string()).unwrap_or_default();
            }
        }
    }

    push_track_info(&mut result, current_track_info, current_has_description);

    if result.is_empty() {
        anyhow::bail!("No MIDs found in offer SDP");
    }

    Ok(result)
}

fn push_track_info(
    result: &mut Vec<InboundTrackInfo>,
    track_info: Option<InboundTrackInfo>,
    has_description: bool,
) {
    if let Some(track_info) =
        track_info.filter(|track_info| has_description && !track_info.mid.is_empty())
    {
        result.push(track_info);
    }
}

fn parse_muted(value: Option<&str>) -> bool {
    matches!(value, Some("1"))
}

pub(crate) fn set_track_info(
    mut sdp: RTCSessionDescription,
    track_info: Vec<&OutboundTrackInfo>,
) -> Result<RTCSessionDescription, anyhow::Error> {
    let mut session = sdp.unmarshal()?;

    for media in session.media_descriptions.iter_mut() {
        if media.media_name.media.eq_ignore_ascii_case("application") {
            continue;
        }

        let Some(msid) = media.attribute("msid").flatten() else {
            continue;
        };

        let Some(track) = track_info.iter().find(|t| t.msid == msid) else {
            continue;
        };

        let purpose_str = track.purpose.to_string();
        let muted_str = format_muted(track.muted);
        let description = format!("{} {} {}", purpose_str, muted_str, track.info.as_str());
        media.media_title = Some(description)
    }

    log::trace!("updated session: {session:#?}");
    sdp.sdp = session.marshal();
    Ok(sdp)
}

fn format_muted(muted: bool) -> &'static str {
    if muted { "1" } else { "2" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_msid_before_finalizing_media_section() {
        let sdp = "m=video 9 UDP/TLS/RTP/SAVPF 96\r\n\
                   a=mid:1\r\n\
                   a=msid:participant-stream participant-video\r\n\
                   i=1 2 Participant\r\n\
                   m=video 9 UDP/TLS/RTP/SAVPF 96\r\n\
                   a=mid:2\r\n\
                   a=msid:stream-source stream-video\r\n\
                   i=2 1 Livestream\r\n";

        let tracks = parse_offered_track_info(sdp).expect("track metadata");

        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].mid, "1");
        assert_eq!(tracks[0].source_stream_id, "participant-stream");
        assert_eq!(tracks[0].source_track_id, "participant-video");
        assert_eq!(tracks[1].mid, "2");
        assert_eq!(tracks[1].source_stream_id, "stream-source");
        assert_eq!(tracks[1].source_track_id, "stream-video");
    }
}
