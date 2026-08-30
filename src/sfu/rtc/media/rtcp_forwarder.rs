//! An interceptor that surfaces inbound RTCP to the application via `poll_read()`.
//!
//! The default interceptor chain consumes inbound RTCP (for NACK, reports, TWCC, etc.), so
//! it never reaches the application's `poll_read()`. The SFU, however, needs to see a
//! subscriber's keyframe requests (PLI/FIR) about a forwarded stream in order to relay them
//! to the publisher. Installed as the outermost layer, this interceptor queues a copy of
//! every inbound RTCP packet for the application while still passing the original down the
//! chain for the default processing.

use log::trace;
use rtc::interceptor::{interceptor, Interceptor, Packet, StreamInfo, TaggedPacket};
use rtc::rtcp::payload_feedbacks::full_intra_request::FullIntraRequest;
use rtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use rtc::rtcp::transport_feedbacks::transport_layer_nack::TransportLayerNack;
use rtc::rtcp::Packet as RtcpPacket;
use rtc::shared::error::Error;
use std::collections::VecDeque;

use crate::metrics;

/// Builder for [`RtcpForwarderInterceptor`], plugged into a `Registry` via `.with(...)`.
pub(crate) struct RtcpForwarderBuilder<P> {
    role: &'static str,
    diagnostics_enabled: bool,
    _phantom: std::marker::PhantomData<P>,
}

impl<P> Default for RtcpForwarderBuilder<P> {
    fn default() -> Self {
        Self {
            role: "unknown",
            diagnostics_enabled: false,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<P> RtcpForwarderBuilder<P> {
    pub(crate) fn new(role: &'static str, diagnostics_enabled: bool) -> Self {
        Self {
            role,
            diagnostics_enabled,
            _phantom: std::marker::PhantomData,
        }
    }

    pub(crate) fn build(self) -> impl FnOnce(P) -> RtcpForwarderInterceptor<P> {
        move |inner| RtcpForwarderInterceptor::new(inner, self.role, self.diagnostics_enabled)
    }
}

/// Copies inbound RTCP to the application's `poll_read()` without consuming it from the
/// normal interceptor chain.
#[derive(Interceptor)]
pub(crate) struct RtcpForwarderInterceptor<P> {
    #[next]
    next: P,
    role: &'static str,
    diagnostics_enabled: bool,
    read_queue: VecDeque<TaggedPacket>,
}

impl<P> RtcpForwarderInterceptor<P> {
    fn new(next: P, role: &'static str, diagnostics_enabled: bool) -> Self {
        Self {
            next,
            role,
            diagnostics_enabled,
            read_queue: VecDeque::new(),
        }
    }
}

#[interceptor]
impl<P: Interceptor> RtcpForwarderInterceptor<P> {
    #[overrides]
    fn handle_read(&mut self, msg: TaggedPacket) -> Result<(), Self::Error> {
        // Surface only keyframe requests (PLI/FIR) to the application — those are the RTCP
        // the SFU relays upstream to publishers. Everything else (SR/RR/NACK/TWCC) is left
        // to the default chain and not duplicated to poll_read.
        if let Packet::Rtcp(rtcp_packets) = &msg.message {
            if self.diagnostics_enabled {
                for packet in rtcp_packets {
                    if packet.as_any().is::<TransportLayerNack>() {
                        metrics::inc_rtc_nack_cache(self.role, "rtcp_in", "unknown", "nack");
                    }
                }
            }

            let keyframe_requests: Vec<Box<dyn RtcpPacket>> = rtcp_packets
                .iter()
                .filter(|packet| {
                    let any = packet.as_any();
                    any.is::<PictureLossIndication>() || any.is::<FullIntraRequest>()
                })
                .map(|packet| packet.cloned())
                .collect();
            if !keyframe_requests.is_empty() {
                trace!(
                    "RtcpForwarder: surfacing {} PLI/FIR from {} to the application",
                    keyframe_requests.len(),
                    msg.transport.peer_addr
                );
                self.read_queue.push_back(TaggedPacket {
                    now: msg.now,
                    transport: msg.transport,
                    message: Packet::Rtcp(keyframe_requests),
                });
            }
        }
        // Always pass the original down the chain for normal processing.
        self.next.handle_read(msg)
    }

    #[overrides]
    fn bind_local_stream(&mut self, info: &StreamInfo) {
        if self.diagnostics_enabled {
            let media = media_label(info);
            let nack = if stream_supports_nack(info) {
                "nack"
            } else {
                "no_nack"
            };
            let rtx = if info.ssrc_rtx.is_some() && info.payload_type_rtx.is_some() {
                "rtx"
            } else {
                "no_rtx"
            };
            metrics::inc_rtc_nack_cache(self.role, "bind_local_stream", media, nack);
            metrics::inc_rtc_nack_cache(self.role, "bind_local_stream", media, rtx);
        }
        self.next.bind_local_stream(info);
    }

    #[overrides]
    fn unbind_local_stream(&mut self, info: &StreamInfo) {
        if self.diagnostics_enabled {
            metrics::inc_rtc_nack_cache(
                self.role,
                "unbind_local_stream",
                media_label(info),
                "stream",
            );
        }
        self.next.unbind_local_stream(info);
    }

    #[overrides]
    fn poll_read(&mut self) -> Option<Self::Rout> {
        if let Some(pkt) = self.read_queue.pop_front() {
            return Some(pkt);
        }
        self.next.poll_read()
    }

    #[overrides]
    fn close(&mut self) -> Result<(), Self::Error> {
        self.read_queue.clear();
        self.next.close()
    }
}

fn media_label(info: &StreamInfo) -> &'static str {
    if info.mime_type.starts_with("audio/") {
        "audio"
    } else if info.mime_type.starts_with("video/") {
        "video"
    } else {
        "unknown"
    }
}

fn stream_supports_nack(info: &StreamInfo) -> bool {
    info.rtcp_feedback
        .iter()
        .any(|feedback| feedback.typ == "nack" && feedback.parameter.is_empty())
}
