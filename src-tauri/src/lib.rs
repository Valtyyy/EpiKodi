mod bootstrap;
mod commands;

use om_core::{AppEvent, EventBus, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, WindowService};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Services first, window after: the window only appears once every
            // service is available in state.
            let config_dir = app.path().app_config_dir()?;
            bootstrap::register(app, bootstrap::build(&config_dir));

            let decorations = app.state::<WindowService>().get().decorations;
            let window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .fullscreen(false)
                    .decorations(decorations)
                    .inner_size(1280.0, 720.0)
                    .min_inner_size(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)
                    .build()?;

            // Hot-apply window settings: no restart needed (C7).
            let mut events = app.state::<EventBus>().subscribe();
            tauri::async_runtime::spawn(async move {
                while let Ok(event) = events.recv().await {
                    if let AppEvent::WindowSettingsChanged(s) = event
                        && let Err(e) = window.set_decorations(s.decorations)
                    {
                        eprintln!("cannot apply window decorations: {e}");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping::ping,
            commands::window::get_window_settings,
            commands::window::set_window_decorations
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
