use derive_more::Display;
use webrtc::Error as WebRTCError;

pub type MediaResult<T> = Result<T, MediaError>;

#[derive(Debug, Display)]
pub(crate) enum MediaError {
    #[display(fmt = "WebRTC error: {}", _0)]
    WebRTC(WebRTCError),
    #[display(fmt = "SDP State error: {}", _0)]
    SdpState(String),
}

impl MediaError {}

impl From<WebRTCError> for MediaError {
    fn from(e: WebRTCError) -> Self {
        MediaError::WebRTC(e)
    }
}

#[allow(dead_code)]
pub type DataChannelResult<T> = Result<T, MediaError>;
