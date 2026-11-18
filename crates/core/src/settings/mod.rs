//! Typed, validated, hot-applied settings (ARCHITECTURE §9).

mod catalog;
mod model;
mod service;
mod view;

pub use catalog::Settings;
pub use model::{
    ApplyMoment, SettingDescriptor, SettingKind, SettingSource, SettingValue, SettingWarning,
    SettingsPatch,
};
pub use service::{Environment, SettingsMeta, SettingsRepository, SettingsService, env_var_name};
pub use view::SettingsView;
