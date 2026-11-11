//! Example service proving the DI wiring end to end.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{AppError, AppEvent, Clock, EventBus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PingReply {
    pub message: String,
    pub timestamp_ms: u64,
}

#[derive(Clone)]
pub struct PingService {
    clock: Arc<dyn Clock>,
    events: EventBus,
}

impl PingService {
    pub fn new(clock: Arc<dyn Clock>, events: EventBus) -> Self {
        Self { clock, events }
    }

    pub fn ping(&self, name: &str) -> Result<PingReply, AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::validation("name", "Name must not be empty")
                .with_suggestion("Type a name and try again"));
        }
        self.events.publish(AppEvent::Pinged {
            name: name.to_owned(),
        });
        Ok(PingReply {
            message: format!("Hello, {name}!"),
            timestamp_ms: self.clock.now_ms(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::ManualClock;

    fn service() -> (PingService, Arc<ManualClock>, EventBus) {
        let clock = Arc::new(ManualClock::new(1_000));
        let bus = EventBus::new();
        (PingService::new(clock.clone(), bus.clone()), clock, bus)
    }

    #[test]
    fn ping_replies_with_clock_time() {
        let (svc, clock, _) = service();
        clock.advance(500);
        let r = svc.ping("Ada").unwrap();
        assert_eq!(r.message, "Hello, Ada!");
        assert_eq!(r.timestamp_ms, 1_500);
    }

    #[test]
    fn empty_name_is_rejected_with_field() {
        let (svc, _, _) = service();
        let e = svc.ping("   ").unwrap_err();
        assert_eq!(e.field.as_deref(), Some("name"));
        assert_eq!(e.code, "validation");
    }

    #[test]
    fn ping_publishes_event() {
        let (svc, _, bus) = service();
        let mut rx = bus.subscribe();
        svc.ping("Ada").unwrap();
        assert_eq!(
            rx.try_recv().unwrap(),
            AppEvent::Pinged { name: "Ada".into() }
        );
    }
}
