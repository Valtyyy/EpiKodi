//! Composition root: the only file that knows both services and implementations.

use std::sync::Arc;

use infra::SystemClock;
use om_core::{EventBus, PingService};
use tauri::Manager;

/// Every service of the application. Add new services here.
pub struct Services {
    pub ping: PingService,
}

/// Builds the service graph with the real port implementations.
pub fn build() -> Services {
    let events = EventBus::new();
    let clock = Arc::new(SystemClock);

    Services {
        ping: PingService::new(clock, events),
    }
}

/// Puts every service in Tauri state. The destructuring has no `..` on purpose:
/// a new field in `Services` that is not managed here does not compile.
pub fn register<R: tauri::Runtime>(app: &impl Manager<R>, services: Services) {
    let Services { ping } = services;

    app.manage(ping);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_is_managed_after_register() {
        let app = tauri::test::mock_app();
        register(&app, build());

        // Extend this list for every service added to `Services`.
        assert!(app.try_state::<PingService>().is_some(), "PingService");
    }
}
