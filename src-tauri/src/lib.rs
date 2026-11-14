mod bootstrap;
mod commands;

use om_core::SettingsService;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// Smallest inner size the window can be resized to, in logical pixels.
const MIN_WINDOW_WIDTH: f64 = 960.0;
const MIN_WINDOW_HEIGHT: f64 = 540.0;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Services first, window after: the window only appears once every
            // service is available in state.
            let data_dir = app.path().app_data_dir()?;
            let services = tauri::async_runtime::block_on(bootstrap::build(&data_dir));
            bootstrap::register(app, services);

            let mut view = app.state::<SettingsService>().view();
            let window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .fullscreen(false)
                    .decorations(view.current().window_decorations)
                    .inner_size(1280.0, 720.0)
                    .min_inner_size(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)
                    .build()?;

            // Hot-apply `interface.window_decorations`: no restart needed (C7).
            tauri::async_runtime::spawn(async move {
                let mut applied = view.current().window_decorations;
                loop {
                    let wanted = view.changed().await.window_decorations;
                    if wanted != applied {
                        match window.set_decorations(wanted) {
                            Ok(()) => applied = wanted,
                            Err(e) => eprintln!("cannot apply window decorations: {e}"),
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping::ping,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::set_secret,
            commands::settings::clear_secret
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
