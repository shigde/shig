use crate::sfu::media::track_info::{InboundTrackInfo, OutboundTrackInfo};
use crate::sfu::media::{MediaPurpose, RtpCodecType};
use webrtc::peer_connection::RTCSessionDescription;

pub(crate) fn parse_offered_track_info(sdp: &str) -> anyhow::Result<Vec<InboundTrackInfo>> {
    let mut result = Vec::new();
    let mut current_track_info: Option<InboundTrackInfo> = None;
    let mut parsed_mid = false;
    let mut parsed_dec = false;

    for line in sdp.lines() {
        let line = line.trim();
        if line.starts_with("m=audio") {
            current_track_info = Some(InboundTrackInfo::new(RtpCodecType::Audio));
        } else if line.starts_with("m=video") {
            current_track_info = Some(InboundTrackInfo::new(RtpCodecType::Video));
        } else if line.starts_with("a=mid:") {
            if let Some(track_info) = current_track_info.as_mut() {
                track_info.mid = line.trim_start_matches("a=mid:").trim().to_string();
                parsed_mid = true;
            }
        } else if line.starts_with("i=") {
            if let Some(track_info) = current_track_info.as_mut() {
                parsed_dec = true;
                let desc = line.trim_start_matches("i=").trim();
                let mut parts = desc.splitn(3, ' ');

                track_info.purpose = MediaPurpose::from_option_str(parts.next());
                track_info.muted = parse_muted(parts.next());
                track_info.info = parts.next().map(|s| s.to_string()).unwrap_or_default();
            }
        }

        if parsed_mid && parsed_dec {
            if let Some(track_info) = current_track_info.take() {
                result.push(track_info);
            }
            parsed_mid = false;
            parsed_dec = false;
            current_track_info = None;
        }
    }

    if result.is_empty() {
        anyhow::bail!("No MIDs found in offer SDP");
    }

    Ok(result)
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
    if muted {
        "1"
    } else {
        "2"
    }
}
