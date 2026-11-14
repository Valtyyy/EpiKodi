//! Typed, validated, hot-applied settings (ARCHITECTURE §9).

mod catalog;
mod model;
mod service;
mod view;

pub use catalog::Settings;
pub use model::{ApplyMoment, SettingDescriptor, SettingKind, SettingValue, SettingsPatch};
pub use service::{SettingsRepository, SettingsService};
pub use view::SettingsView;
