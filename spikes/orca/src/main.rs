// Minimal Tauri 2 host for the Orca / AT-SPI spike (spike 2.5, issue #14).
// The UI is a static page in `dist/`; all accessibility behaviour lives in the page.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running the Orca spike");
}
