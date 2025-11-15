#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::sfu::media::data_channel::DataChannelMsg;
    use webrtc::data_channel::data_channel_message::DataChannelMessage;
    #[test]
    fn test_from_data_channel_message_bin_offer() {
        // The JSON string that expects
        let json = r#"{
            "type": "1",
            "data": {
                "number": 42,
                "sdp": "v=0 test sdp"
            }
        }"#;

        // DataChannelMessage simulieren (binary)
        let dcm = DataChannelMessage {
            data: Bytes::from(json.as_bytes().to_vec()),
            is_string: false,
        };

        // Decode
        let msg = DataChannelMsg::from_data_channel_message_bin(&dcm).expect("should decode");

        match msg {
            DataChannelMsg::OfferMsg(data) => {
                assert_eq!(data.number, 42);
                assert_eq!(data.sdp, "v=0 test sdp");
            }
            other => panic!("expected OfferMsg, got {:?}", other),
        }
    }

    #[test]
    fn test_from_data_channel_message_bin_wrong_type() {
        let dcm = DataChannelMessage {
            data: Bytes::from_static(b"{}"),
            is_string: true,
        };

        let err = DataChannelMsg::from_data_channel_message_bin(&dcm)
            .unwrap_err()
            .to_string();

        assert!(err.contains("Expected binary"));
    }
}
