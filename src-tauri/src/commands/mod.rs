pub mod parse;
pub mod tasks;
pub mod files;
pub mod settings;
pub mod history;

use tauri::ipc::Invoke;

/// Register all Tauri invoke command handlers.
pub fn register_commands() -> impl Fn(Invoke) -> bool {
    tauri::generate_handler![
        parse::parse_links,
        tasks::add_download_task,
        tasks::pause_task,
        tasks::resume_task,
        tasks::cancel_task,
        tasks::retry_task,
        files::open_file,
        files::open_folder,
        settings::get_settings,
        settings::update_settings,
        settings::select_directory,
        settings::read_clipboard,
        history::get_history,
        history::clear_history,
    ]
}
