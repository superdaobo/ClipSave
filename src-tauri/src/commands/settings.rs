use tauri::command;
use crate::error::AppError;
use crate::models::AppSettings;

/// Get the current application settings.
#[command]
pub async fn get_settings() -> Result<AppSettings, AppError> {
    // TODO: Implement in Task 9 - read from SQLite
    Ok(AppSettings::default())
}

/// Update application settings with validation.
#[command]
pub async fn update_settings(settings: AppSettings) -> Result<(), AppError> {
    // Validate max_concurrency range
    if settings.max_concurrency < 1 || settings.max_concurrency > 8 {
        return Err(AppError::InvalidInput {
            message: "max_concurrency must be between 1 and 8".to_string(),
        });
    }

    // TODO: Implement in Task 9 - persist to SQLite
    let _ = settings;
    Ok(())
}

/// Open a directory selection dialog and return the chosen path.
#[command]
pub async fn select_directory() -> Result<Option<String>, AppError> {
    // TODO: Implement with tauri-plugin-dialog
    Ok(None)
}

/// Read the current clipboard text content (explicit invocation only).
#[command]
pub async fn read_clipboard() -> Result<String, AppError> {
    // TODO: Implement with tauri-plugin-clipboard-manager
    Ok(String::new())
}
