use tauri::command;
use crate::error::AppError;
use crate::models::HistoryEntry;

/// Get download history with optional search and status filter.
#[command]
pub async fn get_history(
    search: Option<String>,
    status_filter: Option<String>,
) -> Result<Vec<HistoryEntry>, AppError> {
    // TODO: Implement in Task 9 - query from SQLite
    let _ = (search, status_filter);
    Ok(vec![])
}

/// Clear all history entries (requires frontend confirmation before calling).
#[command]
pub async fn clear_history() -> Result<(), AppError> {
    // TODO: Implement in Task 9 - delete from SQLite
    Ok(())
}
