use tauri::{WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Bootstrap registration (`bootstrap.rs`, services managed in state)
            // will run here, before the window is created (next step), so that
            // the window only appears once every service is available.
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .fullscreen(true)
                .decorations(false)
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
