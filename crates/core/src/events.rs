//! Internal event bus: lets a service react to another without depending on it.

use tokio::sync::broadcast;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Pinged { name: String },
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
}

impl EventBus {
    const CAPACITY: usize = 256;

    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(Self::CAPACITY);
        Self { tx }
    }

    /// Publishes an event. No subscriber is not an error.
    pub fn publish(&self, event: AppEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
