//! Domain services, ports and events. No Tauri, GTK, libmpv or network dependency.

pub mod clock;
pub mod error;
pub mod events;
pub mod input;
pub mod ping;
pub mod secrets;
pub mod settings;
pub mod testing;
pub mod ui;

pub use clock::Clock;
pub use error::AppError;
pub use events::{AppEvent, EventBus};
pub use input::{InputAction, InputService};
pub use ping::{PingReply, PingService};
pub use secrets::{
    SecretError, SecretKey, SecretSource, SecretState, SecretStatus, SecretStore, SecretValue,
};
pub use settings::{
    ApplyMoment, Environment, SettingChange, SettingDescriptor, SettingEntry, SettingKind,
    SettingSource, SettingValue, SettingWarning, Settings, SettingsMeta, SettingsPatch,
    SettingsRepository, SettingsService, SettingsSnapshot, SettingsView, env_var_name,
};
pub use ui::{UiBridge, UiEvent};
