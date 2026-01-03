use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;

#[derive(Debug, Clone)]
pub struct OfferedMid {
    pub mid: String,
    pub kind: RTPCodecType,
}

pub fn parse_offered_mids(sdp: &str) -> anyhow::Result<Vec<OfferedMid>> {
    let mut result = Vec::new();
    let mut current_kind: Option<RTPCodecType> = None;

    for line in sdp.lines() {
        if line.starts_with("m=audio") {
            current_kind = Some(RTPCodecType::Audio);
        } else if line.starts_with("m=video") {
            current_kind = Some(RTPCodecType::Video);
        } else if line.starts_with("a=mid:") {
            let kind = current_kind
                .clone()
                .ok_or_else(|| anyhow::anyhow!("a=mid without preceding m="))?;

            let mid = line.trim_start_matches("a=mid:").trim().to_string();

            result.push(OfferedMid { mid, kind });
        }
    }

    if result.is_empty() {
        anyhow::bail!("No MIDs found in offer SDP");
    }

    Ok(result)
}
