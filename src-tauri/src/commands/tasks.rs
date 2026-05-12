use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tauri::{command, AppHandle, Emitter};
use tokio::sync::{mpsc, watch};

use crate::download::downloader::{download_file, DownloadConfig, DownloadProgress};
use crate::download::task_queue::TaskQueue;
use crate::error::AppError;
use crate::models::{
    DownloadTask, MediaType, Platform, TaskCompletedEvent, TaskFailedEvent, TaskProgressEvent,
    TaskStatus,
};

/// Global task queue singleton.
static TASK_QUEUE: OnceLock<Arc<TaskQueue>> = OnceLock::new();

fn get_queue() -> &'static Arc<TaskQueue> {
    TASK_QUEUE.get_or_init(|| Arc::new(TaskQueue::new(3)))
}

/// Add a resolved media item to the download queue and start processing.
#[command]
pub async fn add_download_task(
    app: AppHandle,
    url: String,
    platform: String,
) -> Result<String, AppError> {
    let plat = match platform.as_str() {
        "douyin" => Platform::Douyin,
        "xiaohongshu" => Platform::Xiaohongshu,
        _ => Platform::Douyin,
    };

    // Determine media type from URL
    let media_type = guess_media_type(&url);

    let task = DownloadTask::new(url, plat, media_type);
    let task_id = task.id.clone();

    let queue = get_queue().clone();
    queue.enqueue(task).await;

    // Spawn queue processing
    let app_clone = app.clone();
    let queue_clone = queue.clone();
    tokio::spawn(process_next_task(app_clone, queue_clone));

    Ok(task_id)
}

/// Process the next available task in the queue.
fn process_next_task(
    app: AppHandle,
    queue: Arc<TaskQueue>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(async move {
    // Try to promote the next waiting task
    let task_id = match queue.promote_next().await {
        Some(id) => id,
        None => return, // No tasks to process or at capacity
    };

    // Get the task details
    let task = match queue.get_task(&task_id).await {
        Some(t) => t,
        None => return,
    };

    // Transition to downloading
    if let Err(e) = queue.update_status(&task_id, TaskStatus::Downloading).await {
        tracing::error!("Failed to transition task {} to downloading: {}", task_id, e);
        return;
    }

    // Emit status update to frontend
    let _ = app.emit(
        "task-status-changed",
        serde_json::json!({
            "id": task_id,
            "status": "downloading"
        }),
    );

    // Determine save path
    let download_dir = get_download_dir();
    // Ensure download directory exists
    if let Err(e) = tokio::fs::create_dir_all(&download_dir).await {
        tracing::error!("Failed to create download dir: {}", e);
        let _ = queue.update_status(&task_id, TaskStatus::Failed).await;
        let _ = app.emit("task-failed", TaskFailedEvent {
            id: task_id.clone(),
            error_code: "DiskFullOrIoError".to_string(),
            error_message: format!("无法创建下载目录: {}", e),
        });
        return;
    }

    let filename = generate_filename(&task);
    let save_path = download_dir.join(&filename);

    tracing::info!("Starting download: {} -> {:?}", task.url.chars().take(80).collect::<String>(), save_path);

    // Create cancel channel
    let (_cancel_tx, cancel_rx) = watch::channel(false);
    let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(32);

    // For DASH_MERGE URLs, use the full URL; for normal URLs, use as-is
    let download_url = task.url.clone();

    let config = DownloadConfig {
        url: download_url,
        save_path: save_path.clone(),
        max_retries: 3,
        initial_backoff_ms: 1000,
        max_backoff_ms: 30000,
        timeout_secs: 120,
    };

    // Spawn progress reporter
    let app_progress = app.clone();
    let progress_task_id = task_id.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let _ = app_progress.emit(
                "task-progress",
                TaskProgressEvent {
                    id: progress_task_id.clone(),
                    progress: progress.progress_percent,
                    speed: progress.speed,
                    downloaded_size: progress.downloaded,
                    total_size: progress.total,
                },
            );
        }
    });

    // Execute download
    match download_file(config, cancel_rx, progress_tx).await {
        Ok(final_path) => {
            tracing::info!("Download completed: {:?}", final_path);
            let _ = queue.update_status(&task_id, TaskStatus::Completed).await;
            let _ = app.emit(
                "task-completed",
                TaskCompletedEvent {
                    id: task_id.clone(),
                    save_path: final_path.to_string_lossy().to_string(),
                },
            );
        }
        Err(e) => {
            tracing::error!("Download failed for task {}: {}", task_id, e);
            let _ = queue.update_status(&task_id, TaskStatus::Failed).await;
            let _ = app.emit(
                "task-failed",
                TaskFailedEvent {
                    id: task_id.clone(),
                    error_code: "DownloadFailed".to_string(),
                    error_message: format!("下载失败: {}", e),
                },
            );
        }
    }

    // Try to process next task in queue
    let app_next = app.clone();
    let queue_next = queue.clone();
    tokio::spawn(process_next_task(app_next, queue_next));
    })
}

/// Get the download directory.
/// Defaults to system Downloads folder if not configured.
/// On Android/iOS, this would be the app's documents or gallery directory.
fn get_download_dir() -> PathBuf {
    // Try system Downloads directory first
    if let Some(downloads) = dirs::download_dir() {
        return downloads.join("ClipSave");
    }

    // Fallback: try home directory
    if let Some(home) = dirs::home_dir() {
        return home.join("Downloads").join("ClipSave");
    }

    // Last resort: current directory
    PathBuf::from("ClipSave_Downloads")
}

/// Generate a filename for the download task based on media type and URL.
/// Saves into a subfolder named after the title.
fn generate_filename(task: &DownloadTask) -> PathBuf {
    // Determine file extension from media type and URL
    let ext = guess_extension(&task.url, &task.media_type);

    // Use title as folder name
    let title = task
        .title
        .as_deref()
        .unwrap_or("未知标题");

    // Sanitize folder name
    let folder_name: String = title
        .chars()
        .take(60)
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' || c > '\u{2E80}' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let folder_name = folder_name.trim();
    let folder_name = if folder_name.is_empty() { "未知标题" } else { folder_name };

    // Generate file name
    let file_base = format!("{}_{}", folder_name.chars().take(30).collect::<String>(), &task.id[..8]);

    let filename = format!("{}.{}", file_base.trim(), ext);

    PathBuf::from(folder_name).join(filename)
}

/// Guess the file extension from URL and media type.
fn guess_extension(url: &str, media_type: &MediaType) -> &'static str {
    // Check URL for hints
    if url.contains(".jpg") || url.contains("jpeg") || url.contains("image/jpeg") {
        return "jpg";
    }
    if url.contains(".png") || url.contains("image/png") {
        return "png";
    }
    if url.contains(".gif") || url.contains("image/gif") {
        return "gif";
    }
    if url.contains(".webp") || url.contains("image/webp") {
        return "webp";
    }
    if url.contains(".mp4") || url.contains("video/mp4") || url.contains("mime_type=video_mp4") {
        return "mp4";
    }

    // Fall back to media type
    match media_type {
        MediaType::Video => "mp4",
        MediaType::Image => "jpg",
        MediaType::Gif => "gif",
        MediaType::Unknown => "bin",
    }
}

/// Guess media type from URL patterns.
fn guess_media_type(url: &str) -> MediaType {
    if url.starts_with("DASH_MERGE||") || url.contains("video") || url.contains(".mp4") || url.contains("mime_type=video_mp4") {
        MediaType::Video
    } else if url.contains(".gif") || url.contains("format/gif") {
        MediaType::Gif
    } else if url.contains(".jpg") || url.contains(".png") || url.contains(".webp") || url.contains("image") || url.contains("xhscdn") {
        MediaType::Image
    } else {
        MediaType::Unknown
    }
}

/// Pause a running download task.
#[command]
pub async fn pause_task(id: String) -> Result<(), AppError> {
    let queue = get_queue();
    queue.update_status(&id, TaskStatus::Paused).await
}

/// Resume a paused download task.
#[command]
pub async fn resume_task(app: AppHandle, id: String) -> Result<(), AppError> {
    let queue = get_queue().clone();
    queue.update_status(&id, TaskStatus::Waiting).await?;

    tokio::spawn(process_next_task(app, queue));

    Ok(())
}

/// Cancel a download task and clean up partial files.
#[command]
pub async fn cancel_task(id: String) -> Result<(), AppError> {
    let queue = get_queue();
    queue.update_status(&id, TaskStatus::Cancelled).await
}

/// Retry a failed download task.
#[command]
pub async fn retry_task(app: AppHandle, id: String) -> Result<(), AppError> {
    let queue = get_queue().clone();
    queue.update_status(&id, TaskStatus::Waiting).await?;

    tokio::spawn(process_next_task(app, queue));

    Ok(())
}
