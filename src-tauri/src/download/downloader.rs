use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use reqwest::header::{CONTENT_LENGTH, RANGE};

use crate::error::AppError;

/// Download configuration.
pub struct DownloadConfig {
    pub url: String,
    pub save_path: PathBuf,
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub timeout_secs: u64,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            save_path: PathBuf::new(),
            max_retries: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 30000,
            timeout_secs: 120,
        }
    }
}

/// Progress callback data.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: u64,
    pub progress_percent: f64,
}

/// Download a file with resume support, retry, and progress reporting.
///
/// - Writes to a `.part` temporary file
/// - Atomically renames to final path on completion
/// - Supports HTTP Range for resume
/// - Retries with exponential backoff on transient errors
/// - Cancellable via the cancel_rx channel
pub async fn download_file(
    config: DownloadConfig,
    cancel_rx: watch::Receiver<bool>,
    progress_tx: tokio::sync::mpsc::Sender<DownloadProgress>,
) -> Result<PathBuf, AppError> {
    // Check if this is a DASH merge request (video + audio separate streams)
    if config.url.starts_with("DASH_MERGE||") {
        return download_and_merge_dash(config, cancel_rx, progress_tx).await;
    }

    let part_path = config.save_path.with_extension(
        format!(
            "{}.part",
            config
                .save_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
        ),
    );

    let mut retries = 0;
    let mut backoff = config.initial_backoff_ms;

    loop {
        if *cancel_rx.borrow() {
            // Clean up partial file on cancel
            let _ = fs::remove_file(&part_path).await;
            return Err(AppError::NetworkError {
                message: "Download cancelled".to_string(),
            });
        }

        match attempt_download(&config, &part_path, &cancel_rx, &progress_tx).await {
            Ok(_) => {
                // Atomic rename from .part to final path
                fs::rename(&part_path, &config.save_path).await.map_err(|e| {
                    AppError::DiskFullOrIoError {
                        message: format!("Failed to rename file: {}", e),
                    }
                })?;
                return Ok(config.save_path);
            }
            Err(e) => {
                if is_transient_error(&e) && retries < config.max_retries {
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                    backoff = (backoff * 2).min(config.max_backoff_ms);
                    continue;
                }
                // Clean up on permanent failure
                let _ = fs::remove_file(&part_path).await;
                return Err(e);
            }
        }
    }
}

/// Attempt a single download, supporting resume via Range header.
async fn attempt_download(
    config: &DownloadConfig,
    part_path: &PathBuf,
    cancel_rx: &watch::Receiver<bool>,
    progress_tx: &tokio::sync::mpsc::Sender<DownloadProgress>,
) -> Result<(), AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| AppError::NetworkError {
            message: format!("Failed to create client: {}", e),
        })?;

    // Check existing partial download size for resume
    let existing_size = if part_path.exists() {
        fs::metadata(part_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    let mut request = client.get(&config.url);

    // Add Referer header based on the CDN domain (required by some CDNs)
    if config.url.contains("xhscdn.com") || config.url.contains("xiaohongshu.com") {
        request = request.header("Referer", "https://www.xiaohongshu.com/");
    } else if config.url.contains("douyinvod.com") || config.url.contains("douyin.com") {
        request = request.header("Referer", "https://www.douyin.com/");
    }

    if existing_size > 0 {
        request = request.header(RANGE, format!("bytes={}-", existing_size));
    }

    let response = request.send().await.map_err(AppError::from)?;
    let status = response.status();

    if !status.is_success() && status.as_u16() != 206 {
        return Err(AppError::NetworkError {
            message: format!("HTTP {} - 服务器拒绝请求", status),
        });
    }

    let total_size = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .map(|content_len| content_len + existing_size);

    // Open file for append (resume) or create
    let mut file = if existing_size > 0 && status.as_u16() == 206 {
        fs::OpenOptions::new()
            .append(true)
            .open(part_path)
            .await
            .map_err(|e| AppError::DiskFullOrIoError {
                message: format!("Failed to open file for resume: {}", e),
            })?
    } else {
        // Ensure parent directory exists
        if let Some(parent) = part_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::DiskFullOrIoError {
                    message: format!("Failed to create directory: {}", e),
                }
            })?;
        }
        fs::File::create(part_path).await.map_err(|e| {
            AppError::DiskFullOrIoError {
                message: format!("Failed to create file: {}", e),
            }
        })?
    };

    let mut downloaded = existing_size;
    let mut stream = response.bytes_stream();
    let mut last_progress_time = Instant::now();
    let mut speed_samples: Vec<(Instant, u64)> = Vec::new();

    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        if *cancel_rx.borrow() {
            return Err(AppError::NetworkError {
                message: "Download cancelled".to_string(),
            });
        }

        let chunk = chunk.map_err(|e| AppError::NetworkError {
            message: format!("Stream error: {}", e),
        })?;

        file.write_all(&chunk).await.map_err(|e| {
            AppError::DiskFullOrIoError {
                message: format!("Write error: {}", e),
            }
        })?;

        downloaded += chunk.len() as u64;
        speed_samples.push((Instant::now(), chunk.len() as u64));

        // Report progress at most every 250ms
        let now = Instant::now();
        if now.duration_since(last_progress_time) >= Duration::from_millis(250) {
            // Calculate speed from rolling 5-second window
            let cutoff = now - Duration::from_secs(5);
            speed_samples.retain(|(t, _)| *t > cutoff);
            let speed: u64 = speed_samples.iter().map(|(_, s)| s).sum::<u64>()
                / speed_samples
                    .first()
                    .map(|(t, _)| now.duration_since(*t).as_secs().max(1))
                    .unwrap_or(1);

            let progress_percent = total_size
                .map(|total| (downloaded as f64 / total as f64) * 100.0)
                .unwrap_or(0.0);

            let _ = progress_tx
                .send(DownloadProgress {
                    downloaded,
                    total: total_size,
                    speed,
                    progress_percent,
                })
                .await;

            last_progress_time = now;
        }
    }

    file.flush().await.map_err(|e| AppError::DiskFullOrIoError {
        message: format!("Flush error: {}", e),
    })?;

    Ok(())
}

/// Check if an error is transient and worth retrying.
fn is_transient_error(err: &AppError) -> bool {
    matches!(err, AppError::NetworkError { .. } | AppError::TimeoutError { .. })
}

/// Download separate video and audio DASH streams, then merge them with ffmpeg.
/// URL format: "DASH_MERGE||{video_url}||{audio_url}"
async fn download_and_merge_dash(
    config: DownloadConfig,
    cancel_rx: watch::Receiver<bool>,
    progress_tx: tokio::sync::mpsc::Sender<DownloadProgress>,
) -> Result<PathBuf, AppError> {
    // Parse the DASH_MERGE URL format
    let parts: Vec<&str> = config.url.split("||").collect();
    if parts.len() != 3 {
        return Err(AppError::InvalidInput {
            message: "Invalid DASH_MERGE URL format".to_string(),
        });
    }
    let video_url = parts[1].to_string();
    let audio_url = parts[2].to_string();

    let video_path = config.save_path.with_extension("video.part");
    let audio_path = config.save_path.with_extension("audio.part");

    // Download video stream
    let video_config = DownloadConfig {
        url: video_url,
        save_path: video_path.clone(),
        max_retries: config.max_retries,
        initial_backoff_ms: config.initial_backoff_ms,
        max_backoff_ms: config.max_backoff_ms,
        timeout_secs: config.timeout_secs,
    };

    // Download video
    let _ = progress_tx.send(DownloadProgress {
        downloaded: 0,
        total: None,
        speed: 0,
        progress_percent: 0.0,
    }).await;

    download_single_stream(&video_config, &video_path, &cancel_rx, &progress_tx).await?;

    if *cancel_rx.borrow() {
        let _ = fs::remove_file(&video_path).await;
        return Err(AppError::NetworkError { message: "Download cancelled".to_string() });
    }

    // Download audio stream
    let _ = progress_tx.send(DownloadProgress {
        downloaded: 0,
        total: None,
        speed: 0,
        progress_percent: 50.0, // Video done, audio starting
    }).await;

    let audio_config = DownloadConfig {
        url: audio_url,
        save_path: audio_path.clone(),
        max_retries: config.max_retries,
        initial_backoff_ms: config.initial_backoff_ms,
        max_backoff_ms: config.max_backoff_ms,
        timeout_secs: config.timeout_secs,
    };

    download_single_stream(&audio_config, &audio_path, &cancel_rx, &progress_tx).await?;

    if *cancel_rx.borrow() {
        let _ = fs::remove_file(&video_path).await;
        let _ = fs::remove_file(&audio_path).await;
        return Err(AppError::NetworkError { message: "Download cancelled".to_string() });
    }

    // Merge video + audio using ffmpeg
    let _ = progress_tx.send(DownloadProgress {
        downloaded: 0,
        total: None,
        speed: 0,
        progress_percent: 90.0, // Merging
    }).await;

    merge_with_ffmpeg(&video_path, &audio_path, &config.save_path).await?;

    // Clean up temp files
    let _ = fs::remove_file(&video_path).await;
    let _ = fs::remove_file(&audio_path).await;

    let _ = progress_tx.send(DownloadProgress {
        downloaded: 0,
        total: None,
        speed: 0,
        progress_percent: 100.0,
    }).await;

    Ok(config.save_path)
}

/// Download a single stream to a file (used by DASH merge).
async fn download_single_stream(
    config: &DownloadConfig,
    part_path: &PathBuf,
    cancel_rx: &watch::Receiver<bool>,
    progress_tx: &tokio::sync::mpsc::Sender<DownloadProgress>,
) -> Result<(), AppError> {
    let mut retries = 0;
    let mut backoff = config.initial_backoff_ms;

    loop {
        if *cancel_rx.borrow() {
            let _ = fs::remove_file(part_path).await;
            return Err(AppError::NetworkError { message: "Download cancelled".to_string() });
        }

        match attempt_download(config, part_path, cancel_rx, progress_tx).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                if is_transient_error(&e) && retries < config.max_retries {
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                    backoff = (backoff * 2).min(config.max_backoff_ms);
                    continue;
                }
                let _ = fs::remove_file(part_path).await;
                return Err(e);
            }
        }
    }
}

/// Merge video and audio files using the built-in pure Rust MP4 muxer.
/// No external ffmpeg dependency — works on all platforms (Windows, Android, iOS).
/// Falls back to just copying the video if merging fails.
async fn merge_with_ffmpeg(
    video_path: &PathBuf,
    audio_path: &PathBuf,
    output_path: &PathBuf,
) -> Result<(), AppError> {
    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| AppError::DiskFullOrIoError {
            message: format!("Failed to create output directory: {}", e),
        })?;
    }

    // Use our built-in pure Rust MP4 muxer (no ffmpeg needed)
    let v_path = video_path.clone();
    let a_path = audio_path.clone();
    let o_path = output_path.clone();

    // Run the CPU-intensive mux operation on a blocking thread
    let result = tokio::task::spawn_blocking(move || {
        crate::download::muxer::merge_video_audio(&v_path, &a_path, &o_path)
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            // Muxer failed — fall back to video-only
            tracing::warn!("MP4 muxer failed: {}, falling back to video-only", e);
            fs::copy(video_path, output_path).await.map_err(|e| {
                AppError::DiskFullOrIoError {
                    message: format!("Failed to copy video file: {}", e),
                }
            })?;
            Ok(())
        }
        Err(e) => {
            // Task panicked — fall back to video-only
            tracing::warn!("Mux task panicked: {}, falling back to video-only", e);
            fs::copy(video_path, output_path).await.map_err(|e| {
                AppError::DiskFullOrIoError {
                    message: format!("Failed to copy video file: {}", e),
                }
            })?;
            Ok(())
        }
    }
}
