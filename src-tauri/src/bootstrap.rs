//! Composition root: the only file that knows both services and implementations.

use std::path::Path;
use std::sync::Arc;

use infra::{JsonWindowStore, SystemClock};
use om_core::{EventBus, PingService, WindowService};
use tauri::Manager;

/// Every service of the application. Add new services here.
pub struct Services {
    pub events: EventBus,
    pub ping: PingService,
    pub window: WindowService,
}

/// Builds the service graph with the real port implementations.
pub fn build(config_dir: &Path) -> Services {
    let events = EventBus::new();
    let clock = Arc::new(SystemClock);
    let window_store = Arc::new(JsonWindowStore::new(config_dir.join("window.json")));

    Services {
        ping: PingService::new(clock, events.clone()),
        window: WindowService::new(window_store, events.clone()),
        events,
    }
}

/// Puts every service in Tauri state. The destructuring has no `..` on purpose:
/// a new field in `Services` that is not managed here does not compile.
pub fn register<R: tauri::Runtime>(app: &impl Manager<R>, services: Services) {
    let Services {
        events,
        ping,
        window,
    } = services;

    app.manage(events);
    app.manage(ping);
    app.manage(window);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_is_managed_after_register() {
        let app = tauri::test::mock_app();
        register(&app, build(&std::env::temp_dir().join("om-bootstrap-test")));

        // Extend this list for every service added to `Services`.
        assert!(app.try_state::<EventBus>().is_some(), "EventBus");
        assert!(app.try_state::<PingService>().is_some(), "PingService");
        assert!(app.try_state::<WindowService>().is_some(), "WindowService");
    }
}
