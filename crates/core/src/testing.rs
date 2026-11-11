//! In-memory fakes of ports, reusable by other crates' tests.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{AppError, Clock, WindowSettings, WindowSettingsStore};

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

/// In-memory `WindowSettingsStore`.
#[derive(Default)]
pub struct MemoryWindowStore(Mutex<Option<WindowSettings>>);

impl MemoryWindowStore {
    pub fn with(settings: WindowSettings) -> Self {
        Self(Mutex::new(Some(settings)))
    }

    pub fn saved(&self) -> Option<WindowSettings> {
        *self.0.lock().unwrap()
    }
}

impl WindowSettingsStore for MemoryWindowStore {
    fn load(&self) -> Result<Option<WindowSettings>, AppError> {
        Ok(self.saved())
    }

    fn save(&self, settings: &WindowSettings) -> Result<(), AppError> {
        *self.0.lock().unwrap() = Some(*settings);
        Ok(())
    }
}
