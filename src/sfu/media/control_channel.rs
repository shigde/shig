use crate::sfu::media::message::ControlChannelMessage;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use webrtc::data_channel::RTCDataChannel;

pub struct ControlChannel {
    session: Mutex<String>,
    queue: Mutex<VecDeque<ControlChannelMessage>>,
    sender: Mutex<Option<mpsc::Sender<ControlChannelMessage>>>,
}

impl ControlChannel {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            session: Mutex::new(Uuid::new_v4().to_string()),
            queue: Mutex::new(VecDeque::new()),
            sender: Mutex::new(None),
        })
    }

    pub async fn attach_channel(&self, tx: mpsc::Sender<ControlChannelMessage>) {
        let mut session = self.session.lock().await;
        *session = Uuid::new_v4().to_string();

        *self.sender.lock().await = Some(tx);

        // Flush Queue
        let mut queue = self.queue.lock().await;
        while let Some(msg) = queue.pop_front() {
            let _ = self.send_raw(msg).await;
        }
    }

    pub async fn detach_channel(&self) {
        *self.sender.lock().await = None;
    }

    pub async fn send(&self, kind: &str, payload: serde_json::Value) {
        let msg = ControlChannelMessage {
            id: Uuid::new_v4().to_string(),
            session: self.session.lock().await.clone(),
            kind: kind.into(),
            payload,
        };

        if self.send_raw(msg.clone()).await.is_err() {
            self.queue.lock().await.push_back(msg);
        }
    }

    async fn send_raw(&self, msg: ControlChannelMessage) -> Result<(), ()> {
        if let Some(tx) = self.sender.lock().await.as_ref() {
            tx.send(msg).await.map_err(|_| ())?;
            Ok(())
        } else {
            Err(())
        }
    }
}

pub async fn bind_datachannel(dc: Arc<RTCDataChannel>, messenger: Arc<ControlChannel>) {
    let (tx, mut rx) = mpsc::channel::<ControlChannelMessage>(64);
    messenger.attach_channel(tx).await;
    // Outgoing
    let dc_out = dc.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let _ = dc_out.send_text(serde_json::to_string(&msg).unwrap()).await;
        }
    });

    // Incoming
    let m_in = messenger.clone();
    dc.on_message(Box::new(move |msg| {
        let m = m_in.clone();
        Box::pin(async move {
            if let Ok(text) = std::str::from_utf8(&msg.data) {
                if let Ok(msg) = serde_json::from_str::<ControlChannelMessage>(text) {
                    println!("{:?}", msg)
                }
            }
        })
    }));

    // Close
    let m_close = messenger.clone();
    dc.on_close(Box::new(move || {
        let m = m_close.clone();
        Box::pin(async move {
            println!("DataChannel closed");
            m.detach_channel().await;
        })
    }));
}
