//! Window preferences: optional title bar (window manager decorations).

use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::{AppError, AppEvent, EventBus};

/// Smallest inner size the window can be resized to, in logical pixels.
pub const MIN_WINDOW_WIDTH: f64 = 960.0;
pub const MIN_WINDOW_HEIGHT: f64 = 540.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WindowSettings {
    /// Show the window manager title bar.
    pub decorations: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self { decorations: true }
    }
}

/// Persistence port for [`WindowSettings`].
pub trait WindowSettingsStore: Send + Sync {
    /// `Ok(None)` when nothing was saved yet.
    fn load(&self) -> Result<Option<WindowSettings>, AppError>;
    fn save(&self, settings: &WindowSettings) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct WindowService {
    store: Arc<dyn WindowSettingsStore>,
    events: EventBus,
    current: Arc<Mutex<WindowSettings>>,
}

impl WindowService {
    /// A corrupt or unreadable store falls back to the defaults: the window
    /// must always be able to open.
    pub fn new(store: Arc<dyn WindowSettingsStore>, events: EventBus) -> Self {
        let current = store.load().ok().flatten().unwrap_or_default();
        Self {
            store,
            events,
            current: Arc::new(Mutex::new(current)),
        }
    }

    pub fn get(&self) -> WindowSettings {
        *self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Saves, then publishes `WindowSettingsChanged` so the shell applies it
    /// without a restart.
    pub fn set_decorations(&self, decorations: bool) -> Result<WindowSettings, AppError> {
        let mut current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        let next = WindowSettings { decorations };
        if *current != next {
            self.store.save(&next)?;
            *current = next;
            self.events.publish(AppEvent::WindowSettingsChanged(next));
        }
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MemoryWindowStore;

    fn service(store: Arc<MemoryWindowStore>) -> (WindowService, EventBus) {
        let bus = EventBus::new();
        (WindowService::new(store, bus.clone()), bus)
    }

    #[test]
    fn defaults_to_decorated() {
        let (svc, _) = service(Arc::new(MemoryWindowStore::default()));
        assert!(svc.get().decorations);
    }

    #[test]
    fn restores_saved_choice() {
        let store = Arc::new(MemoryWindowStore::with(WindowSettings {
            decorations: false,
        }));
        let (svc, _) = service(store);
        assert!(!svc.get().decorations);
    }

    #[test]
    fn set_persists_and_publishes() {
        let store = Arc::new(MemoryWindowStore::default());
        let (svc, bus) = service(store.clone());
        let mut rx = bus.subscribe();

        svc.set_decorations(false).unwrap();

        assert!(!svc.get().decorations);
        assert!(!store.saved().unwrap().decorations);
        assert_eq!(
            rx.try_recv().unwrap(),
            AppEvent::WindowSettingsChanged(WindowSettings { decorations: false })
        );
    }

    #[test]
    fn unchanged_value_is_a_no_op() {
        let (svc, bus) = service(Arc::new(MemoryWindowStore::default()));
        let mut rx = bus.subscribe();
        svc.set_decorations(true).unwrap();
        assert!(rx.try_recv().is_err());
    }
}
