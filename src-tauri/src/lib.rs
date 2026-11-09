// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::str::FromStr;

struct UserService {
    name: String,
    age: i16
}

impl UserService {

    pub fn new(name: &str, age: i16) -> UserService {
        UserService { name: String::from_str(name).unwrap(), age: age }
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let service = UserService::new("John", 24);

    service.age;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
