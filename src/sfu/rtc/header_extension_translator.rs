use bytes::Bytes;
use rtc::rtp_transceiver::rtp_sender::RTCRtpHeaderExtensionParameters;
use rtc_sdp::extmap::{SDES_MID_URI, TRANSPORT_CC_URI};
use rtp::header::{EXTENSION_PROFILE_ONE_BYTE, EXTENSION_PROFILE_TWO_BYTE};
use rtp::packet::Packet;
use std::collections::HashMap;

/// Cached source-to-target extension rewrites for one subscriber RTP leg.
/// The publisher and subscriber negotiate extmap IDs independently. The RTP
/// payload remains shared; only this per-subscriber packet header copy is rewritten.
#[derive(Clone)]
pub(crate) struct RtpHeaderExtensionTranslator {
    rules: HashMap<u8, HeaderExtensionRewrite>,
}

#[derive(Clone)]
enum HeaderExtensionRewrite {
    Copy { target_id: u8 },
    Mid { target_id: u8, target_mid: Bytes },
}

impl RtpHeaderExtensionTranslator {
    pub(crate) fn new(
        source_extensions: &HashMap<u8, String>,
        target_extensions: &[RTCRtpHeaderExtensionParameters],
        target_mid: String,
    ) -> Self {
        let target_by_uri: HashMap<&str, u8> = target_extensions
            .iter()
            .filter_map(|extension| {
                u8::try_from(extension.id)
                    .ok()
                    .map(|id| (extension.uri.as_str(), id))
            })
            .collect();
        let target_mid = Bytes::from(target_mid);
        let rules = source_extensions
            .iter()
            .filter_map(|(source_id, uri)| {
                if uri == TRANSPORT_CC_URI {
                    // The target leg's TWCC sender interceptor assigns its own sequence number.
                    return None;
                }

                let target_id = *target_by_uri.get(uri.as_str())?;
                let rewrite = if uri == SDES_MID_URI {
                    HeaderExtensionRewrite::Mid {
                        target_id,
                        target_mid: target_mid.clone(),
                    }
                } else {
                    HeaderExtensionRewrite::Copy { target_id }
                };
                Some((*source_id, rewrite))
            })
            .collect();

        Self { rules }
    }

    pub(crate) fn translate(&self, packet: &mut Packet) {
        packet.header.extensions.retain_mut(|extension| {
            let Some(rewrite) = self.rules.get(&extension.id) else {
                return false;
            };

            match rewrite {
                HeaderExtensionRewrite::Copy { target_id } => extension.id = *target_id,
                HeaderExtensionRewrite::Mid {
                    target_id,
                    target_mid,
                } => {
                    extension.id = *target_id;
                    extension.payload = target_mid.clone();
                }
            }
            true
        });

        if packet.header.extensions.is_empty() {
            packet.header.extension = false;
            packet.header.extension_profile = 0;
            packet.header.extensions_padding = 0;
        } else if packet.header.extension_profile == EXTENSION_PROFILE_ONE_BYTE
            && packet
                .header
                .extensions
                .iter()
                .any(|extension| extension.id > 14 || extension.payload.len() > 16)
        {
            packet.header.extension_profile = EXTENSION_PROFILE_TWO_BYTE;
        }
    }
}
