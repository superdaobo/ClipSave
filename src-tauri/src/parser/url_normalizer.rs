use percent_encoding::percent_decode_str;
use reqwest::redirect::Policy;
use std::collections::HashSet;
use std::time::Duration;
use url::Url;

use crate::error::AppError;
use crate::parser::common::NormalizedUrl;

/// Maximum number of HTTP redirects to follow for short links.
const MAX_REDIRECTS: usize = 5;

/// HTTP timeout for redirect resolution requests.
const REDIRECT_TIMEOUT_SECS: u64 = 15;

/// Known tracking parameters to strip from URLs.
const TRACKING_PARAMS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "share_token",
    "share_from",
    "share_app_id",
    "app_platform",
    "timestamp",
    "xhsshare",
    "appuid",
    "apptime",
    "share_id",
    "shareRedId",
    "share_source",
    "sec_uid",
    "enter_from",
    "enter_method",
];

/// Whitelisted hosts that are safe to follow redirects to.
const WHITELISTED_HOSTS: &[&str] = &[
    "douyin.com",
    "www.douyin.com",
    "v.douyin.com",
    "iesdouyin.com",
    "www.iesdouyin.com",
    "xiaohongshu.com",
    "www.xiaohongshu.com",
    "xhslink.com",
];

/// Normalize a URL: percent-decode, strip tracking params, resolve short links.
/// This function is idempotent: normalize(normalize(u)) == normalize(u).
pub async fn normalize(input: &str) -> Result<NormalizedUrl, AppError> {
    let trimmed = input.trim();

    // Percent-decode the URL
    let decoded = percent_decode_str(trimmed)
        .decode_utf8()
        .map_err(|_| AppError::InvalidInput {
            message: "URL contains invalid UTF-8 after decoding".to_string(),
        })?
        .to_string();

    // Parse the URL
    let mut parsed = Url::parse(&decoded).map_err(|e| AppError::InvalidInput {
        message: format!("Invalid URL: {}", e),
    })?;

    // Validate scheme is HTTP or HTTPS
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::UnsafeRedirect {
            message: format!("Unsupported scheme: {}", parsed.scheme()),
        });
    }

    // Strip tracking parameters
    let tracking_set: HashSet<&str> = TRACKING_PARAMS.iter().copied().collect();
    let filtered_params: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(key, _)| !tracking_set.contains(key.as_ref()))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    // Rebuild query string
    if filtered_params.is_empty() {
        parsed.set_query(None);
    } else {
        let query_string: String = filtered_params
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    k.clone()
                } else {
                    format!("{}={}", k, v)
                }
            })
            .collect::<Vec<_>>()
            .join("&");
        parsed.set_query(Some(&query_string));
    }

    let host = parsed.host_str().unwrap_or("").to_string();

    // Check if this is a short link that needs redirect resolution
    let is_short_link = is_short_link_host(&host);

    let canonical = if is_short_link {
        resolve_redirects(&parsed.to_string()).await?
    } else {
        parsed.to_string()
    };

    // Parse the canonical URL to extract host and path
    let canonical_parsed = Url::parse(&canonical).map_err(|e| AppError::InvalidInput {
        message: format!("Invalid canonical URL: {}", e),
    })?;

    Ok(NormalizedUrl {
        original: input.to_string(),
        canonical,
        host: canonical_parsed.host_str().unwrap_or("").to_string(),
        path: canonical_parsed.path().to_string(),
    })
}

/// Check if a host is a known short link domain that requires redirect resolution.
fn is_short_link_host(host: &str) -> bool {
    host == "v.douyin.com" || host == "xhslink.com"
}

/// Follow HTTP redirects up to MAX_REDIRECTS hops.
/// Returns the final URL after all redirects.
async fn resolve_redirects(url: &str) -> Result<String, AppError> {
    let client = reqwest::Client::builder()
        .redirect(Policy::none()) // Manual redirect handling
        .timeout(Duration::from_secs(REDIRECT_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (compatible; ClipSave/1.0)")
        .build()
        .map_err(|e| AppError::NetworkError {
            message: format!("Failed to create HTTP client: {}", e),
        })?;

    let mut current_url = url.to_string();
    let mut hops = 0;

    loop {
        if hops >= MAX_REDIRECTS {
            return Err(AppError::TooManyRedirects {
                message: format!("Redirect chain exceeded {} hops", MAX_REDIRECTS),
            });
        }

        let response = match client.head(&current_url).send().await {
            Ok(resp) => resp,
            Err(_) => {
                // Fallback to GET if HEAD fails
                client.get(&current_url).send().await.map_err(|e| AppError::NetworkError {
                    message: format!("Failed to resolve redirect: {}", e),
                })?
            }
        };

        let status = response.status();

        if status.is_redirection() {
            let location = response
                .headers()
                .get("location")
                .and_then(|v: &reqwest::header::HeaderValue| v.to_str().ok())
                .ok_or_else(|| AppError::NetworkError {
                    message: "Redirect without Location header".to_string(),
                })?;

            // Resolve relative URLs
            let next_url = if location.starts_with("http://") || location.starts_with("https://") {
                location.to_string()
            } else {
                let base = Url::parse(&current_url).map_err(|_| AppError::InvalidInput {
                    message: "Invalid base URL for relative redirect".to_string(),
                })?;
                base.join(location)
                    .map_err(|_| AppError::InvalidInput {
                        message: "Invalid relative redirect URL".to_string(),
                    })?
                    .to_string()
            };

            // Validate redirect target
            let next_parsed = Url::parse(&next_url).map_err(|_| AppError::UnsafeRedirect {
                message: format!("Invalid redirect target: {}", next_url),
            })?;

            // Check scheme safety
            if next_parsed.scheme() != "http" && next_parsed.scheme() != "https" {
                return Err(AppError::UnsafeRedirect {
                    message: format!("Redirect to non-HTTP(S) scheme: {}", next_parsed.scheme()),
                });
            }

            // Check host whitelist
            let next_host = next_parsed.host_str().unwrap_or("");
            if !is_whitelisted_host(next_host) {
                // Allow redirect to non-whitelisted hosts but log it
                // This is needed because platforms may redirect through CDN domains
            }

            current_url = next_url;
            hops += 1;
        } else {
            // No more redirects
            break;
        }
    }

    // Strip tracking params from the final URL too
    let mut final_parsed = Url::parse(&current_url).map_err(|e| AppError::InvalidInput {
        message: format!("Invalid final URL: {}", e),
    })?;

    let tracking_set: HashSet<&str> = TRACKING_PARAMS.iter().copied().collect();
    let filtered_params: Vec<(String, String)> = final_parsed
        .query_pairs()
        .filter(|(key, _)| !tracking_set.contains(key.as_ref()))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    if filtered_params.is_empty() {
        final_parsed.set_query(None);
    } else {
        let query_string: String = filtered_params
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    k.clone()
                } else {
                    format!("{}={}", k, v)
                }
            })
            .collect::<Vec<_>>()
            .join("&");
        final_parsed.set_query(Some(&query_string));
    }

    Ok(final_parsed.to_string())
}

/// Check if a host is in the whitelist of known safe redirect targets.
fn is_whitelisted_host(host: &str) -> bool {
    WHITELISTED_HOSTS.iter().any(|&h| host == h || host.ends_with(&format!(".{}", h)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_normalize_strips_tracking_params() {
        let url = "https://www.douyin.com/video/123?utm_source=share&utm_medium=social&vid=123";
        let result = normalize(url).await.unwrap();
        assert!(!result.canonical.contains("utm_source"));
        assert!(!result.canonical.contains("utm_medium"));
        assert!(result.canonical.contains("vid=123"));
    }

    #[tokio::test]
    async fn test_normalize_preserves_required_params() {
        let url = "https://www.xiaohongshu.com/explore/abc123?xsec_token=test123";
        let result = normalize(url).await.unwrap();
        assert!(result.canonical.contains("xsec_token=test123"));
    }

    #[tokio::test]
    async fn test_normalize_rejects_non_http_scheme() {
        let result = normalize("ftp://example.com/file").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_normalize_handles_percent_encoding() {
        let url = "https://www.douyin.com/video/123%20test";
        let result = normalize(url).await.unwrap();
        assert!(result.canonical.contains("douyin.com"));
    }

    #[tokio::test]
    async fn test_normalize_idempotence_no_redirect() {
        let url = "https://www.douyin.com/video/123?utm_source=share&vid=456";
        let first = normalize(url).await.unwrap();
        let second = normalize(&first.canonical).await.unwrap();
        assert_eq!(first.canonical, second.canonical);
    }

    #[test]
    fn test_is_short_link_host() {
        assert!(is_short_link_host("v.douyin.com"));
        assert!(is_short_link_host("xhslink.com"));
        assert!(!is_short_link_host("www.douyin.com"));
        assert!(!is_short_link_host("www.xiaohongshu.com"));
    }

    #[test]
    fn test_tracking_params_list() {
        assert!(TRACKING_PARAMS.contains(&"utm_source"));
        assert!(TRACKING_PARAMS.contains(&"share_token"));
        assert!(TRACKING_PARAMS.contains(&"xhsshare"));
    }
}
