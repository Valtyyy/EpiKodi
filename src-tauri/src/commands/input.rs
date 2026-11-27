use om_core::{AppError, InputAction, InputService};
use tauri::State;

/// Same path as the phone's `POST /api/v1/input`: through `InputService` and back to
/// the interface as a `UiEvent`.
#[tauri::command]
pub async fn send_input(
    action: InputAction,
    state: State<'_, InputService>,
) -> Result<(), AppError> {
    state.send(action);
    Ok(())
}
