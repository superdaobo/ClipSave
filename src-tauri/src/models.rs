use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

/// Supported platforms for media resolution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Douyin,
    Xiaohongshu,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Douyin => write!(f, "douyin"),
            Platform::Xiaohongshu => write!(f, "xiaohongshu"),
        }
    }
}

/// Media type classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Video,
    Image,
    Gif,
    Unknown,
}

/// Download task status with state machine semantics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Waiting,
    Parsing,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    /// Check if a transition from the current state to the target state is valid.
    /// Returns Ok(()) if valid, Err(InvalidTransition) if not.
    ///
    /// State machine:
    /// - waiting → parsing | cancelled
    /// - parsing → downloading | failed | cancelled
    /// - downloading → paused | completed | failed | cancelled
    /// - paused → downloading | cancelled
    /// - failed → waiting (retry) | cancelled
    /// - completed and cancelled are terminal states
    pub fn can_transition_to(&self, target: &TaskStatus) -> Result<(), AppError> {
        let valid = match self {
            TaskStatus::Waiting => matches!(target, TaskStatus::Parsing | TaskStatus::Cancelled),
            TaskStatus::Parsing => matches!(
                target,
                TaskStatus::Downloading | TaskStatus::Failed | TaskStatus::Cancelled
            ),
            TaskStatus::Downloading => matches!(
                target,
                TaskStatus::Paused
                    | TaskStatus::Completed
                    | TaskStatus::Failed
                    | TaskStatus::Cancelled
            ),
            TaskStatus::Paused => {
                matches!(target, TaskStatus::Downloading | TaskStatus::Cancelled)
            }
            TaskStatus::Failed => {
                matches!(target, TaskStatus::Waiting | TaskStatus::Cancelled)
            }
            // Terminal states
            TaskStatus::Completed | TaskStatus::Cancelled => false,
        };

        if valid {
            Ok(())
        } else {
            Err(AppError::InvalidTransition {
                message: format!(
                    "Cannot transition from {:?} to {:?}",
                    self, target
                ),
                from: format!("{:?}", self).to_lowercase(),
                to: format!("{:?}", target).to_lowercase(),
            })
        }
    }

    /// Check if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Cancelled)
    }
}

/// A single media item within a resolved media result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub media_type: MediaType,
    pub url: String,
    pub filename_hint: Option<String>,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
    /// Bitrate in kbps (extracted from URL params like `br=`)
    pub bitrate: Option<u32>,
    /// Human-readable quality label (e.g., "1080p", "720p", "480p")
    pub quality_label: Option<String>,
}

/// Result of resolving a URL to its media content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedMedia {
    pub platform: Platform,
    pub source_url: String,
    pub canonical_url: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub media_items: Vec<MediaItem>,
    pub cover: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

/// A download task entity with full state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub platform: Platform,
    pub status: TaskStatus,
    pub progress: f64,
    pub speed: u64,
    pub downloaded_size: u64,
    pub total_size: Option<u64>,
    pub save_path: Option<String>,
    pub error: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub media_type: MediaType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DownloadTask {
    /// Create a new download task in waiting state.
    pub fn new(url: String, platform: Platform, media_type: MediaType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            url,
            platform,
            status: TaskStatus::Waiting,
            progress: 0.0,
            speed: 0,
            downloaded_size: 0,
            total_size: None,
            save_path: None,
            error: None,
            title: None,
            author: None,
            media_type,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Application settings persisted in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub download_dir: String,
    pub max_concurrency: u8,
    pub filename_template: String,
    pub auto_clipboard: bool,
    pub keep_history: bool,
    pub debug_log: bool,
    pub theme: String,
    pub language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            download_dir: String::new(),
            max_concurrency: 3,
            filename_template: "{platform}/{author}/{date}/{title}_{index}.{ext}".to_string(),
            auto_clipboard: false,
            keep_history: true,
            debug_log: false,
            theme: "system".to_string(),
            language: "zh-CN".to_string(),
        }
    }
}

/// A history entry for display in the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub url: String,
    pub platform: Platform,
    pub author: Option<String>,
    pub title: Option<String>,
    pub status: TaskStatus,
    pub save_path: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Tauri event payload for task progress updates.
#[derive(Debug, Clone, Serialize)]
pub struct TaskProgressEvent {
    pub id: String,
    pub progress: f64,
    pub speed: u64,
    pub downloaded_size: u64,
    pub total_size: Option<u64>,
}

/// Tauri event payload for task completion.
#[derive(Debug, Clone, Serialize)]
pub struct TaskCompletedEvent {
    pub id: String,
    pub save_path: String,
}

/// Tauri event payload for task failure.
#[derive(Debug, Clone, Serialize)]
pub struct TaskFailedEvent {
    pub id: String,
    pub error_code: String,
    pub error_message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions_from_waiting() {
        let status = TaskStatus::Waiting;
        assert!(status.can_transition_to(&TaskStatus::Parsing).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Cancelled).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Downloading).is_err());
        assert!(status.can_transition_to(&TaskStatus::Completed).is_err());
    }

    #[test]
    fn test_valid_transitions_from_parsing() {
        let status = TaskStatus::Parsing;
        assert!(status.can_transition_to(&TaskStatus::Downloading).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Failed).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Cancelled).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Paused).is_err());
        assert!(status.can_transition_to(&TaskStatus::Completed).is_err());
    }

    #[test]
    fn test_valid_transitions_from_downloading() {
        let status = TaskStatus::Downloading;
        assert!(status.can_transition_to(&TaskStatus::Paused).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Completed).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Failed).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Cancelled).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Waiting).is_err());
    }

    #[test]
    fn test_valid_transitions_from_paused() {
        let status = TaskStatus::Paused;
        assert!(status.can_transition_to(&TaskStatus::Downloading).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Cancelled).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Completed).is_err());
    }

    #[test]
    fn test_valid_transitions_from_failed() {
        let status = TaskStatus::Failed;
        // Retry goes back to waiting
        assert!(status.can_transition_to(&TaskStatus::Waiting).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Cancelled).is_ok());
        assert!(status.can_transition_to(&TaskStatus::Downloading).is_err());
    }

    #[test]
    fn test_terminal_states_reject_all_transitions() {
        for terminal in [TaskStatus::Completed, TaskStatus::Cancelled] {
            assert!(terminal.can_transition_to(&TaskStatus::Waiting).is_err());
            assert!(terminal.can_transition_to(&TaskStatus::Parsing).is_err());
            assert!(terminal.can_transition_to(&TaskStatus::Downloading).is_err());
            assert!(terminal.can_transition_to(&TaskStatus::Paused).is_err());
            assert!(terminal.can_transition_to(&TaskStatus::Failed).is_err());
            assert!(terminal.can_transition_to(&TaskStatus::Completed).is_err());
            assert!(terminal.can_transition_to(&TaskStatus::Cancelled).is_err());
            assert!(terminal.is_terminal());
        }
    }

    #[test]
    fn test_download_task_creation() {
        let task = DownloadTask::new(
            "https://example.com".to_string(),
            Platform::Douyin,
            MediaType::Video,
        );
        assert_eq!(task.status, TaskStatus::Waiting);
        assert_eq!(task.progress, 0.0);
        assert!(!task.id.is_empty());
    }

    #[test]
    fn test_default_settings() {
        let settings = AppSettings::default();
        assert_eq!(settings.max_concurrency, 3);
        assert_eq!(settings.language, "zh-CN");
        assert_eq!(settings.theme, "system");
        assert!(!settings.auto_clipboard);
        assert!(settings.keep_history);
    }
}
