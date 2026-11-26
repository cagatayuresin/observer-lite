use std::sync::Mutex;

use tokio::sync::mpsc;

const MAX_SUBSCRIBERS: usize = 200;
const QUEUE_DEPTH: usize = 50;

#[derive(Clone, Default)]
pub struct SseHub {
    senders: std::sync::Arc<Mutex<Vec<mpsc::Sender<String>>>>,
}

impl SseHub {
    pub fn subscribe(&self) -> Option<mpsc::Receiver<String>> {
        let mut senders = self.senders.lock().expect("sse hub");
        senders.retain(|sender| !sender.is_closed());
        if senders.len() >= MAX_SUBSCRIBERS {
            return None;
        }
        let (tx, rx) = mpsc::channel(QUEUE_DEPTH);
        senders.push(tx);
        Some(rx)
    }

    pub fn publish(&self, event: &str, data: &serde_json::Value) {
        let payload = format!(
            "event: {event}\ndata: {}\n\n",
            serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string())
        );
        let mut senders = self.senders.lock().expect("sse hub");
        senders.retain(|sender| sender.try_send(payload.clone()).is_ok());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribers_receive_events_until_the_queue_fills() {
        let hub = SseHub::default();
        let mut first = hub.subscribe().unwrap();
        hub.publish("ping", &serde_json::json!({"n": 1}));
        let message = first.try_recv().unwrap();
        assert!(message.starts_with("event: ping\n"));
        drop(first);
        hub.publish("ping", &serde_json::json!({"n": 2}));

        let mut held = Vec::new();
        for _ in 0..MAX_SUBSCRIBERS {
            held.push(hub.subscribe().unwrap());
        }
        assert!(hub.subscribe().is_none());
        for _ in 0..(QUEUE_DEPTH + 2) {
            hub.publish("flood", &serde_json::json!({}));
        }
    }
}
