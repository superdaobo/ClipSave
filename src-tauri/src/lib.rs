pub mod commands;
pub mod download;
pub mod error;
pub mod logger;
pub mod models;
pub mod parser;
pub mod storage;

use commands::register_commands;

/// Initialize and run the Tauri application with all plugins and commands.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(register_commands())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
