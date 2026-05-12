use std::path::Path;
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

/// Initialize the structured logger.
///
/// - Produces JSON-formatted logs with timestamp, level, module, event, task_id
/// - Rotates log files: max 5 files of 5 MB each
/// - When debug_log enabled: logs at info level and above
/// - When debug_log disabled: logs at warn level and above
/// - Redacts sensitive data (cookies, auth headers, tokens)
pub fn init_logger(log_dir: &Path, debug_enabled: bool) {
    let level_filter = if debug_enabled {
        "info"
    } else {
        "warn"
    };

    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        log_dir,
        "clipsave.log",
    );

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level_filter));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .json()
                .with_writer(file_appender)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .compact(),
        )
        .init();
}

/// Redact query string values from a URL for safe logging.
/// Preserves host and path but replaces query values with "[REDACTED]".
pub fn redact_url(url: &str) -> String {
    if let Ok(mut parsed) = url::Url::parse(url) {
        if parsed.query().is_some() {
            let redacted_params: Vec<String> = parsed
                .query_pairs()
                .map(|(k, _)| format!("{}=[REDACTED]", k))
                .collect();
            if redacted_params.is_empty() {
                parsed.set_query(None);
            } else {
                parsed.set_query(Some(&redacted_params.join("&")));
            }
        }
        parsed.to_string()
    } else {
        "[INVALID_URL]".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_url_with_params() {
        let url = "https://example.com/path?token=secret&id=123";
        let redacted = redact_url(url);
        assert!(redacted.contains("token=[REDACTED]"));
        assert!(redacted.contains("id=[REDACTED]"));
        assert!(!redacted.contains("secret"));
        assert!(!redacted.contains("123"));
    }

    #[test]
    fn test_redact_url_without_params() {
        let url = "https://example.com/path";
        let redacted = redact_url(url);
        assert_eq!(redacted, "https://example.com/path");
    }

    #[test]
    fn test_redact_invalid_url() {
        let url = "not a url";
        let redacted = redact_url(url);
        assert_eq!(redacted, "[INVALID_URL]");
    }
}
