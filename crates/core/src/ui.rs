//! Port towards the TV interface (ARCHITECTURE.md section 3, ADR-0001).

use serde::Serialize;

use crate::InputAction;

/// Everything core pushes to the TV interface. Serialised as `{ "type": ..., ... }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum UiEvent {
    /// An input from another device (phone remote), handled like a living-room key.
    Input { action: InputAction },
}

/// Implemented with `AppHandle::emit` in `src-tauri`; services never see Tauri.
pub trait UiBridge: Send + Sync {
    /// Fire and forget: no interface listening is not an error.
    fn send(&self, event: UiEvent);
}
