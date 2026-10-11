use super::header_extension_translator::RtpHeaderExtensionTranslator;
use bytes::Bytes;
use rtc::rtp_transceiver::rtp_sender::RTCRtpHeaderExtensionParameters;
use rtc_sdp::extmap::{SDES_MID_URI, TRANSPORT_CC_URI};
use rtp::header::{EXTENSION_PROFILE_ONE_BYTE, Extension};
use rtp::packet::Packet;
use rtc::shared::marshal::Marshal;
use std::collections::HashMap;

#[test]
fn rewrites_extensions_for_the_subscriber_leg() {
    let source_extensions = HashMap::from([
        (1, SDES_MID_URI.to_string()),
        (3, TRANSPORT_CC_URI.to_string()),
        (4, "urn:example:audio-level".to_string()),
    ]);
    let target_extensions = vec![
        RTCRtpHeaderExtensionParameters {
            uri: SDES_MID_URI.to_string(),
            id: 7,
            encrypted: false,
        },
        RTCRtpHeaderExtensionParameters {
            uri: "urn:example:audio-level".to_string(),
            id: 2,
            encrypted: false,
        },
    ];
    let translator = RtpHeaderExtensionTranslator::new(
        &source_extensions,
        &target_extensions,
        "subscriber-mid".to_string(),
    );
    let mut packet = Packet::default();
    packet.header.extension = true;
    packet.header.extension_profile = EXTENSION_PROFILE_ONE_BYTE;
    // The publisher packet had a three-byte extension padding before TWCC was removed.
    packet.header.extensions_padding = 3;
    packet.header.extensions = vec![
        Extension {
            id: 1,
            payload: Bytes::from_static(b"publisher-mid"),
        },
        Extension {
            id: 3,
            payload: Bytes::from_static(b"publisher-twcc"),
        },
        Extension {
            id: 4,
            payload: Bytes::from_static(b"audio-level"),
        },
    ];

    translator.translate(&mut packet);

    assert_eq!(packet.header.extensions.len(), 2);
    assert_eq!(packet.header.extensions[0].id, 7);
    assert_eq!(packet.header.extensions[0].payload, "subscriber-mid");
    assert_eq!(packet.header.extensions[1].id, 2);
    assert_eq!(packet.header.extensions[1].payload, "audio-level");
    assert_eq!(packet.header.extensions_padding, 1);
    assert!(packet.marshal().is_ok());
}
