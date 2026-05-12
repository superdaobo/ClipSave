use async_trait::async_trait;
use crate::error::AppError;
use crate::models::ResolvedMedia;

/// Normalized URL after cleaning, decoding, and redirect resolution.
#[derive(Debug, Clone)]
pub struct NormalizedUrl {
    pub original: String,
    pub canonical: String,
    pub host: String,
    pub path: String,
}

/// Trait that all platform parsers must implement.
/// Each parser handles a specific set of hosts/URL patterns.
#[async_trait]
pub trait Parser: Send + Sync {
    /// Check if this parser can handle the given URL.
    fn can_handle(&self, url: &str) -> bool;

    /// Resolve a normalized URL to its media content.
    /// Returns ResolvedMedia on success, or an appropriate AppError on failure.
    async fn resolve(&self, url: &NormalizedUrl) -> Result<ResolvedMedia, AppError>;
}
