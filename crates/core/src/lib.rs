//! Domain services, ports and events. No Tauri, GTK, libmpv or network dependency.

pub mod clock;
pub mod error;
pub mod events;
pub mod ping;
pub mod settings;
pub mod testing;
pub mod window;

pub use clock::Clock;
pub use error::AppError;
pub use events::{AppEvent, EventBus};
pub use ping::{PingReply, PingService};
pub use settings::{
    ApplyMoment, SettingDescriptor, SettingKind, SettingValue, Settings, SettingsPatch,
    SettingsRepository, SettingsService, SettingsView,
};
pub use window::{
    MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, WindowService, WindowSettings, WindowSettingsStore,
};
