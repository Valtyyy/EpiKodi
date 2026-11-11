use om_core::{AppError, WindowService, WindowSettings};
use tauri::State;

#[tauri::command]
pub async fn get_window_settings(
    state: State<'_, WindowService>,
) -> Result<WindowSettings, AppError> {
    Ok(state.get())
}

#[tauri::command]
pub async fn set_window_decorations(
    decorations: bool,
    state: State<'_, WindowService>,
) -> Result<WindowSettings, AppError> {
    state.set_decorations(decorations)
}
