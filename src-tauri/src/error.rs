use serde::Serialize;
use std::fmt;

/// Application error taxonomy covering all failure modes.
/// Each variant maps to a user-friendly message via the frontend i18n system.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "code", content = "details")]
pub enum AppError {
    /// Parser could not extract media from the page (platform may have changed).
    ParseFailed { message: String, platform_hint: Option<String> },

    /// No registered parser can handle this URL.
    UnsupportedPlatform { message: String },

    /// Content requires login, payment, or is otherwise restricted.
    RestrictedContent { message: String },

    /// The requested content no longer exists (HTTP 404).
    ContentNotFound { message: String },

    /// Network connectivity issue (DNS, connection refused, etc.).
    NetworkError { message: String },

    /// HTTP request timed out.
    TimeoutError { message: String },

    /// File system permission denied or path traversal attempt.
    PermissionDenied { message: String },

    /// Disk full or other I/O error during file write.
    DiskFullOrIoError { message: String },

    /// Invalid input provided by the user or frontend.
    InvalidInput { message: String },

    /// Short link redirect chain exceeded maximum hops (5).
    TooManyRedirects { message: String },

    /// Redirect target uses non-HTTP(S) scheme or untrusted host.
    UnsafeRedirect { message: String },

    /// Requested state transition is not valid for the current task state.
    InvalidTransition { message: String, from: String, to: String },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::ParseFailed { message, .. } => write!(f, "Parse failed: {}", message),
            AppError::UnsupportedPlatform { message } => write!(f, "Unsupported platform: {}", message),
            AppError::RestrictedContent { message } => write!(f, "Restricted content: {}", message),
            AppError::ContentNotFound { message } => write!(f, "Content not found: {}", message),
            AppError::NetworkError { message } => write!(f, "Network error: {}", message),
            AppError::TimeoutError { message } => write!(f, "Timeout: {}", message),
            AppError::PermissionDenied { message } => write!(f, "Permission denied: {}", message),
            AppError::DiskFullOrIoError { message } => write!(f, "I/O error: {}", message),
            AppError::InvalidInput { message } => write!(f, "Invalid input: {}", message),
            AppError::TooManyRedirects { message } => write!(f, "Too many redirects: {}", message),
            AppError::UnsafeRedirect { message } => write!(f, "Unsafe redirect: {}", message),
            AppError::InvalidTransition { message, from, to } => {
                write!(f, "Invalid transition from {} to {}: {}", from, to, message)
            }
        }
    }
}

impl std::error::Error for AppError {}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            AppError::TimeoutError {
                message: "Request timed out".to_string(),
            }
        } else if err.is_connect() {
            AppError::NetworkError {
                message: "Connection failed".to_string(),
            }
        } else if err.is_redirect() {
            AppError::TooManyRedirects {
                message: "Too many redirects".to_string(),
            }
        } else {
            AppError::NetworkError {
                message: format!("HTTP error: {}", err),
            }
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::DiskFullOrIoError {
            message: format!("Database error: {}", err),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::DiskFullOrIoError {
            message: format!("I/O error: {}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_serialization() {
        let err = AppError::ParseFailed {
            message: "test".to_string(),
            platform_hint: Some("douyin".to_string()),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("ParseFailed"));
        assert!(json.contains("test"));
        assert!(json.contains("douyin"));
    }

    #[test]
    fn test_all_variants_serialize() {
        let variants: Vec<AppError> = vec![
            AppError::ParseFailed { message: "m".into(), platform_hint: None },
            AppError::UnsupportedPlatform { message: "m".into() },
            AppError::RestrictedContent { message: "m".into() },
            AppError::ContentNotFound { message: "m".into() },
            AppError::NetworkError { message: "m".into() },
            AppError::TimeoutError { message: "m".into() },
            AppError::PermissionDenied { message: "m".into() },
            AppError::DiskFullOrIoError { message: "m".into() },
            AppError::InvalidInput { message: "m".into() },
            AppError::TooManyRedirects { message: "m".into() },
            AppError::UnsafeRedirect { message: "m".into() },
            AppError::InvalidTransition { message: "m".into(), from: "a".into(), to: "b".into() },
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert!(!json.is_empty());
            // Verify Display trait works
            let display = format!("{}", variant);
            assert!(!display.is_empty());
        }
    }
}
