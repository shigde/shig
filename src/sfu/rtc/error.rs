use derive_more::Display;

pub type RtcResult<T> = Result<T, RtcError>;

#[derive(Debug, Display)]
pub(crate) enum RtcError {
    #[display(fmt = "Socket error: {}", _0)]
    Socket(String),
    #[display(fmt = "EndpointBuild error: {}", _0)]
    EndpointBuild(String),
}
