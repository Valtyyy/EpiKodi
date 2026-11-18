use om_core::{
    AppError, SecretKey, SecretStatus, SecretValue, SettingsPatch, SettingsService,
    SettingsSnapshot,
};
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, SettingsService>) -> Result<SettingsSnapshot, AppError> {
    Ok(state.snapshot().await)
}

#[tauri::command]
pub async fn update_settings(
    patch: SettingsPatch,
    state: State<'_, SettingsService>,
) -> Result<SettingsSnapshot, AppError> {
    state.update(patch).await?;
    Ok(state.snapshot().await)
}

/// The raw value is wrapped immediately so it cannot be logged by accident.
#[tauri::command]
pub async fn set_secret(
    key: SecretKey,
    value: String,
    state: State<'_, SettingsService>,
) -> Result<SecretStatus, AppError> {
    state.set_secret(key, SecretValue::new(value)).await
}

#[tauri::command]
pub async fn clear_secret(
    key: SecretKey,
    state: State<'_, SettingsService>,
) -> Result<SecretStatus, AppError> {
    state.clear_secret(key).await
}
