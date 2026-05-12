use std::time::Duration;
use reqwest::StatusCode;

use crate::error::AppError;

/// HTTP timeout for metadata/page requests (15 seconds).
const METADATA_TIMEOUT_SECS: u64 = 15;

/// Maximum retries for transient failures.
const MAX_RETRIES: u32 = 2;

/// Initial backoff for retries (1 second).
const INITIAL_BACKOFF_MS: u64 = 1000;

/// Fetch a public web page and return its HTML content.
///
/// - Only requests publicly accessible resources without authentication
/// - Uses a generic, non-deceptive User-Agent header
/// - Retries up to 2 times with exponential backoff on transient failures
/// - Returns RestrictedContent when anti-crawling blocks access
/// - Returns ContentNotFound for HTTP 404
pub async fn fetch_page(url: &str) -> Result<String, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(METADATA_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (compatible; ClipSave/1.0)")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| AppError::NetworkError {
            message: format!("Failed to create HTTP client: {}", e),
        })?;

    let mut retries = 0;
    let mut backoff = INITIAL_BACKOFF_MS;

    loop {
        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status();

                match status {
                    StatusCode::OK => {
                        let body = response.text().await.map_err(|e| AppError::NetworkError {
                            message: format!("Failed to read response body: {}", e),
                        })?;
                        return Ok(body);
                    }
                    StatusCode::NOT_FOUND => {
                        return Err(AppError::ContentNotFound {
                            message: "The requested content was not found (404)".to_string(),
                        });
                    }
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                        return Err(AppError::RestrictedContent {
                            message: format!(
                                "Access denied (HTTP {}). Content may require login or is restricted.",
                                status.as_u16()
                            ),
                        });
                    }
                    // HTTP 451 Unavailable For Legal Reasons
                    s if s.as_u16() == 451 => {
                        return Err(AppError::RestrictedContent {
                            message: "Content unavailable for legal reasons (HTTP 451)".to_string(),
                        });
                    }
                    // 5xx server errors are transient
                    s if s.is_server_error() => {
                        if retries < MAX_RETRIES {
                            retries += 1;
                            tokio::time::sleep(Duration::from_millis(backoff)).await;
                            backoff *= 2;
                            continue;
                        }
                        return Err(AppError::NetworkError {
                            message: format!("Server error after {} retries: HTTP {}", MAX_RETRIES, s.as_u16()),
                        });
                    }
                    // 429 Too Many Requests - rate limited
                    s if s.as_u16() == 429 => {
                        return Err(AppError::RestrictedContent {
                            message: "Rate limited by platform. Please try again later.".to_string(),
                        });
                    }
                    _ => {
                        return Err(AppError::NetworkError {
                            message: format!("Unexpected HTTP status: {}", status.as_u16()),
                        });
                    }
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    if retries < MAX_RETRIES {
                        retries += 1;
                        tokio::time::sleep(Duration::from_millis(backoff)).await;
                        backoff *= 2;
                        continue;
                    }
                    return Err(AppError::TimeoutError {
                        message: format!("Request timed out after {} retries", MAX_RETRIES),
                    });
                }
                if retries < MAX_RETRIES {
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                    backoff *= 2;
                    continue;
                }
                return Err(AppError::NetworkError {
                    message: format!("Network error after {} retries: {}", MAX_RETRIES, e),
                });
            }
        }
    }
}
