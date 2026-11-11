//! In-memory fakes of ports, reusable by other crates' tests.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::Clock;

/// Clock that only moves when told to.
pub struct ManualClock(AtomicU64);

impl ManualClock {
    pub fn new(start_ms: u64) -> Self {
        Self(AtomicU64::new(start_ms))
    }

    pub fn advance(&self, ms: u64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
