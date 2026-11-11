//! Tauri commands: pure delegation to services. One file per domain.
//!
//! Modules stay public: `generate_handler!` needs the hidden items the
//! `#[tauri::command]` macro generates next to each command.

pub mod ping;
pub mod window;
