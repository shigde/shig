//! Deliver client keyframe requests after the standard RTCP interceptors have run.
use rtc::interceptor::{Attribute, Interceptor, Packet, StreamInfo, TaggedPacket};
use rtc::sansio::Protocol;
use rtc::shared::error::Error;
use rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use std::{collections::VecDeque, time::Instant};

#[derive(Default)]
pub(crate) struct SubscriberPli {
    reads: VecDeque<TaggedPacket>,
    writes: VecDeque<TaggedPacket>,
}

pub(crate) fn requests_ssrc(packets: &[Box<dyn rtcp::Packet>], ssrc: u32) -> bool {
    packets.iter().any(|packet| packet.as_any().downcast_ref::<PictureLossIndication>()
        .is_some_and(|pli| pli.media_ssrc == ssrc))
}

impl Protocol<TaggedPacket, TaggedPacket, ()> for SubscriberPli {
    type Rout = TaggedPacket;
    type Wout = TaggedPacket;
    type Eout = ();
    type Error = Error;
    type Time = Instant;

    fn handle_read(&mut self, mut msg: TaggedPacket) -> Result<(), Error> {
        if let Packet::Rtcp(packets) = &mut msg.message.packet {
            packets.retain(|packet| packet.as_any().is::<PictureLossIndication>());
            if packets.is_empty() { return Ok(()); }
            msg.message.add(Attribute::DeliverToApplication);
        }
        self.reads.push_back(msg);
        Ok(())
    }

    fn poll_read(&mut self) -> Option<TaggedPacket> { self.reads.pop_front() }
    fn handle_write(&mut self, msg: TaggedPacket) -> Result<(), Error> {
        self.writes.push_back(msg);
        Ok(())
    }
    fn poll_write(&mut self) -> Option<TaggedPacket> { self.writes.pop_front() }
}

impl Interceptor for SubscriberPli {
    fn bind_local_stream(&mut self, _: &StreamInfo) {}
    fn unbind_local_stream(&mut self, _: &StreamInfo) {}
    fn bind_remote_stream(&mut self, _: &StreamInfo) {}
    fn unbind_remote_stream(&mut self, _: &StreamInfo) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtc::interceptor::AttributedPacket;
    use rtcp::receiver_report::ReceiverReport;

    fn message(packets: Vec<Box<dyn rtcp::Packet>>) -> TaggedPacket {
        TaggedPacket { now: Instant::now(), transport: Default::default(),
            message: AttributedPacket::new(Packet::Rtcp(packets)) }
    }

    #[test]
    fn mixed_feedback_delivers_only_pli_with_original_target_ssrc() {
        let mut interceptor = SubscriberPli::default();
        interceptor.handle_read(message(vec![
            Box::new(ReceiverReport::default()),
            Box::new(PictureLossIndication { sender_ssrc: 10, media_ssrc: 42 }),
        ])).unwrap();
        let output = interceptor.poll_read().unwrap();
        assert!(output.message.has(&Attribute::DeliverToApplication));
        let Packet::Rtcp(packets) = output.message.packet else { panic!("expected RTCP"); };
        assert_eq!(packets.len(), 1);
        assert!(requests_ssrc(&packets, 42));
        assert!(!requests_ssrc(&packets, 43));
        assert!(interceptor.poll_read().is_none());
    }

    #[test]
    fn receiver_reports_are_not_delivered_or_forwarded_upstream() {
        let mut interceptor = SubscriberPli::default();
        interceptor.handle_read(message(vec![Box::new(ReceiverReport::default())])).unwrap();
        assert!(interceptor.poll_read().is_none());
    }

    #[test]
    fn outbound_feedback_is_not_filtered() {
        let mut interceptor = SubscriberPli::default();
        interceptor.handle_write(message(vec![Box::new(ReceiverReport::default())])).unwrap();
        let output = interceptor.poll_write().unwrap();
        assert!(!output.message.has(&Attribute::DeliverToApplication));
        let Packet::Rtcp(packets) = output.message.packet else { panic!("expected RTCP"); };
        assert!(packets[0].as_any().is::<ReceiverReport>());
    }
}
