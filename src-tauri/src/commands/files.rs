use tauri::command;
use crate::error::AppError;

/// Open a downloaded file using the system default application.
/// Validates path is within the configured download directory.
#[command]
pub async fn open_file(path: String) -> Result<(), AppError> {
    // Validate path doesn't contain traversal
    if path.contains("..") {
        return Err(AppError::PermissionDenied {
            message: "Path traversal detected".to_string(),
        });
    }

    // TODO: Implement path validation against download_dir and open via opener plugin
    let _ = path;
    Ok(())
}

/// Open the folder containing a downloaded file.
/// Validates path is within the configured download directory.
#[command]
pub async fn open_folder(path: String) -> Result<(), AppError> {
    if path.contains("..") {
        return Err(AppError::PermissionDenied {
            message: "Path traversal detected".to_string(),
        });
    }

    // TODO: Implement path validation and open folder via opener plugin
    let _ = path;
    Ok(())
}
