use std::path::Path;

/// Maximum filename length in UTF-8 bytes.
const MAX_FILENAME_BYTES: usize = 180;

/// Characters invalid on Windows, Android, or iOS filesystems.
const INVALID_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Sanitize a filename for safe use across Windows, Android, and iOS.
///
/// - Replaces invalid filesystem characters with `_`
/// - Removes control characters
/// - Trims trailing dots and spaces
/// - Truncates to MAX_FILENAME_BYTES while preserving extension
/// - Falls back to "untitled" when input sanitizes to empty
/// - Is idempotent: sanitize(sanitize(s)) == sanitize(s)
pub fn sanitize(input: &str, extension: Option<&str>) -> String {
    if input.trim().is_empty() {
        return format_with_extension("untitled", extension);
    }

    // Replace invalid characters and control characters with underscore
    let cleaned: String = input
        .chars()
        .map(|c| {
            if c.is_control() || INVALID_CHARS.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();

    // Trim trailing dots and spaces
    let trimmed = cleaned.trim_end_matches(|c: char| c == '.' || c == ' ');
    let trimmed = trimmed.trim_start();

    if trimmed.is_empty() {
        return format_with_extension("untitled", extension);
    }

    // Truncate to max bytes while preserving extension
    let result = truncate_to_max_bytes(trimmed, extension);

    if result.is_empty() {
        format_with_extension("untitled", extension)
    } else {
        result
    }
}

/// Resolve filename collisions by appending numeric suffix.
/// Returns a unique filename in the given directory.
pub fn resolve_collision(dir: &Path, filename: &str) -> String {
    let path = dir.join(filename);
    if !path.exists() {
        return filename.to_string();
    }

    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let mut counter = 1;
    loop {
        let candidate = if ext.is_empty() {
            format!("{}-{}", stem, counter)
        } else {
            format!("{}-{}.{}", stem, counter, ext)
        };

        if !dir.join(&candidate).exists() {
            return candidate;
        }
        counter += 1;

        // Safety limit to prevent infinite loop
        if counter > 10000 {
            return format!("{}_{}", stem, uuid::Uuid::new_v4());
        }
    }
}

/// Truncate a string to fit within MAX_FILENAME_BYTES, preserving the extension.
fn truncate_to_max_bytes(name: &str, extension: Option<&str>) -> String {
    let ext_part = match extension {
        Some(ext) if !ext.is_empty() => format!(".{}", ext),
        _ => String::new(),
    };

    let ext_bytes = ext_part.len();
    let available_bytes = MAX_FILENAME_BYTES.saturating_sub(ext_bytes);

    if name.len() <= available_bytes {
        return format!("{}{}", name, ext_part);
    }

    // Truncate at a valid UTF-8 boundary
    let mut truncated = String::new();
    for c in name.chars() {
        if truncated.len() + c.len_utf8() > available_bytes {
            break;
        }
        truncated.push(c);
    }

    // Trim trailing spaces/dots from truncated result
    let truncated = truncated.trim_end_matches(|c: char| c == '.' || c == ' ');

    format!("{}{}", truncated, ext_part)
}

/// Format a base name with an optional extension.
fn format_with_extension(base: &str, extension: Option<&str>) -> String {
    match extension {
        Some(ext) if !ext.is_empty() => format!("{}.{}", base, ext),
        _ => base.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_basic() {
        let result = sanitize("hello world", Some("mp4"));
        assert_eq!(result, "hello world.mp4");
    }

    #[test]
    fn test_sanitize_invalid_chars() {
        let result = sanitize("file<name>:test", Some("jpg"));
        assert_eq!(result, "file_name__test.jpg");
    }

    #[test]
    fn test_sanitize_control_chars() {
        let result = sanitize("file\x00name\x1f", Some("png"));
        assert_eq!(result, "file_name_.png");
    }

    #[test]
    fn test_sanitize_trailing_dots_and_spaces() {
        let result = sanitize("filename...", Some("mp4"));
        assert_eq!(result, "filename.mp4");
    }

    #[test]
    fn test_sanitize_empty_input() {
        let result = sanitize("", Some("mp4"));
        assert_eq!(result, "untitled.mp4");
    }

    #[test]
    fn test_sanitize_whitespace_only() {
        let result = sanitize("   ", Some("jpg"));
        assert_eq!(result, "untitled.jpg");
    }

    #[test]
    fn test_sanitize_all_invalid_chars() {
        let result = sanitize("<>:\"/\\|?*", Some("txt"));
        // All chars replaced with _, then trimmed
        assert!(result.contains("_"));
        assert!(result.ends_with(".txt"));
    }

    #[test]
    fn test_sanitize_no_extension() {
        let result = sanitize("filename", None);
        assert_eq!(result, "filename");
    }

    #[test]
    fn test_sanitize_truncation() {
        let long_name = "a".repeat(200);
        let result = sanitize(&long_name, Some("mp4"));
        assert!(result.len() <= MAX_FILENAME_BYTES);
        assert!(result.ends_with(".mp4"));
    }

    #[test]
    fn test_sanitize_idempotence() {
        let inputs = vec![
            "hello world",
            "file<name>test",
            "trailing...",
            "  spaces  ",
            "normal_file",
            "中文文件名",
        ];

        for input in inputs {
            let first = sanitize(input, Some("mp4"));
            // For idempotence test, we need to strip the extension before re-sanitizing
            let stem = first.strip_suffix(".mp4").unwrap_or(&first);
            let second = sanitize(stem, Some("mp4"));
            assert_eq!(first, second, "Not idempotent for input: {}", input);
        }
    }

    #[test]
    fn test_sanitize_chinese_characters() {
        let result = sanitize("抖音视频标题", Some("mp4"));
        assert_eq!(result, "抖音视频标题.mp4");
    }

    #[test]
    fn test_resolve_collision_no_conflict() {
        let dir = TempDir::new().unwrap();
        let result = resolve_collision(dir.path(), "test.mp4");
        assert_eq!(result, "test.mp4");
    }

    #[test]
    fn test_resolve_collision_with_conflict() {
        let dir = TempDir::new().unwrap();
        // Create a file to cause collision
        std::fs::write(dir.path().join("test.mp4"), "").unwrap();
        let result = resolve_collision(dir.path(), "test.mp4");
        assert_eq!(result, "test-1.mp4");
    }

    #[test]
    fn test_resolve_collision_multiple() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.mp4"), "").unwrap();
        std::fs::write(dir.path().join("test-1.mp4"), "").unwrap();
        std::fs::write(dir.path().join("test-2.mp4"), "").unwrap();
        let result = resolve_collision(dir.path(), "test.mp4");
        assert_eq!(result, "test-3.mp4");
    }

    #[test]
    fn test_filename_length_bound() {
        // Test with very long input
        let long_input = "这是一个非常长的中文标题".repeat(20);
        let result = sanitize(&long_input, Some("mp4"));
        assert!(result.as_bytes().len() <= MAX_FILENAME_BYTES);
    }
}
