use std::path::{Path, PathBuf};
use chrono::Utc;

use crate::download::filename_sanitizer;
use crate::error::AppError;
use crate::models::Platform;

/// Organize a download file into the directory structure:
/// {download_dir}/{platform}/{author_or_unknown}/{YYYY-MM-DD}/{filename}
///
/// Validates that the resulting path is within download_dir (no path traversal).
pub fn organize_path(
    download_dir: &Path,
    platform: &Platform,
    author: Option<&str>,
    title: &str,
    extension: &str,
) -> Result<PathBuf, AppError> {
    let platform_dir = platform.to_string();
    let author_dir = author
        .map(|a| filename_sanitizer::sanitize(a, None))
        .unwrap_or_else(|| "unknown".to_string());
    let date_dir = Utc::now().format("%Y-%m-%d").to_string();
    let filename = filename_sanitizer::sanitize(title, Some(extension));

    let target_dir = download_dir
        .join(&platform_dir)
        .join(&author_dir)
        .join(&date_dir);

    let full_path = target_dir.join(&filename);

    // Validate path is within download_dir (prevent path traversal)
    validate_path_within(download_dir, &full_path)?;

    Ok(full_path)
}

/// Validate that a target path resolves within the allowed base directory.
/// Detects path traversal attempts (`..\`, absolute paths outside base, etc.).
pub fn validate_path_within(base: &Path, target: &Path) -> Result<(), AppError> {
    // Normalize both paths
    let base_canonical = base.to_path_buf();
    let target_str = target.to_string_lossy();

    // Check for obvious traversal patterns
    if target_str.contains("..") {
        return Err(AppError::PermissionDenied {
            message: "Path traversal detected: contains '..'".to_string(),
        });
    }

    // Check that target starts with base
    if !target.starts_with(&base_canonical) {
        return Err(AppError::PermissionDenied {
            message: "Path is outside the allowed download directory".to_string(),
        });
    }

    Ok(())
}

/// Ensure the target directory exists, creating it if necessary.
pub async fn ensure_directory(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            AppError::DiskFullOrIoError {
                message: format!("Failed to create directory: {}", e),
            }
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_organize_path_basic() {
        let download_dir = Path::new("/downloads");
        let result = organize_path(
            download_dir,
            &Platform::Douyin,
            Some("author_name"),
            "video title",
            "mp4",
        );
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("douyin"));
        assert!(path.to_string_lossy().contains("author_name"));
        assert!(path.to_string_lossy().contains("video title.mp4"));
    }

    #[test]
    fn test_organize_path_no_author() {
        let download_dir = Path::new("/downloads");
        let result = organize_path(
            download_dir,
            &Platform::Xiaohongshu,
            None,
            "note title",
            "jpg",
        );
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("unknown"));
    }

    #[test]
    fn test_validate_path_traversal_rejected() {
        let base = Path::new("/downloads");
        let target = Path::new("/downloads/../etc/passwd");
        let result = validate_path_within(base, target);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_within_base() {
        let base = Path::new("/downloads");
        let target = Path::new("/downloads/douyin/author/2024-01-01/video.mp4");
        let result = validate_path_within(base, target);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_outside_base() {
        let base = Path::new("/downloads");
        let target = Path::new("/other/path/file.mp4");
        let result = validate_path_within(base, target);
        assert!(result.is_err());
    }
}
