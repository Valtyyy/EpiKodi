use om_core::{AppError, PingReply, PingService};
use tauri::State;

#[tauri::command]
pub async fn ping(name: String, state: State<'_, PingService>) -> Result<PingReply, AppError> {
    state.ping(&name)
}
