//! Read-only handle on the effective settings.

use std::sync::Arc;

use tokio::sync::watch;

use super::catalog::Settings;

/// What services receive instead of the `SettingsService`: they can read and
/// await changes, but never write.
#[derive(Clone)]
pub struct SettingsView {
    rx: watch::Receiver<Arc<Settings>>,
}

impl SettingsView {
    pub(super) fn new(rx: watch::Receiver<Arc<Settings>>) -> Self {
        Self { rx }
    }

    /// A view that never changes, for tests.
    pub fn fixed(settings: Settings) -> Self {
        let (tx, rx) = watch::channel(Arc::new(settings));
        // Keeping the sender alive would need a field; `changed()` handles the
        // closed channel by pending forever instead.
        drop(tx);
        Self { rx }
    }

    pub fn current(&self) -> Arc<Settings> {
        self.rx.borrow().clone()
    }

    /// Resolves with the new settings after the next update. Pends forever once
    /// the service is gone, so a `select!` loop simply stops being woken.
    pub async fn changed(&mut self) -> Arc<Settings> {
        if self.rx.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
        self.rx.borrow_and_update().clone()
    }
}
