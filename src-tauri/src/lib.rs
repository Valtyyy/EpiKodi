mod bootstrap;
mod commands;

use tauri::{WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Services first, window after: the window only appears once every
            // service is available in state.
            bootstrap::register(app, bootstrap::build());
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .fullscreen(true)
                .decorations(false)
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
