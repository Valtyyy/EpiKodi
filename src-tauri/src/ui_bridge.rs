//! `UiBridge` port over Tauri events: the only place core events become `emit` calls.

use om_core::{UiBridge, UiEvent};
use tauri::{AppHandle, Emitter, Runtime};

/// Tauri event name; `ui/tv/src/api/tauri.ts` listens to the same one.
pub const UI_EVENT: &str = "ui-event";

pub struct TauriUiBridge<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriUiBridge<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> UiBridge for TauriUiBridge<R> {
    fn send(&self, event: UiEvent) {
        // Only fails if the webview is gone (shutdown): nothing left to tell.
        if let Err(e) = self.app.emit(UI_EVENT, &event) {
            eprintln!("cannot emit {UI_EVENT}: {e}");
        }
    }
}
