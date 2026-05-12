use crate::error::AppError;
use crate::models::ResolvedMedia;
use crate::parser::common::Parser;
use crate::parser::douyin::DouyinParser;
use crate::parser::url_normalizer;
use crate::parser::xiaohongshu::XiaohongshuParser;

/// Get all registered parsers in priority order.
fn get_parsers() -> Vec<Box<dyn Parser>> {
    vec![
        Box::new(DouyinParser::new()),
        Box::new(XiaohongshuParser::new()),
    ]
}

/// Resolve a list of extracted URLs to their media content.
/// Iterates registered parsers and selects the first match for each URL.
pub async fn resolve_urls(urls: Vec<String>) -> Result<Vec<ResolvedMedia>, AppError> {
    let parsers = get_parsers();
    let mut results = Vec::new();

    for url_str in urls {
        // Normalize the URL first
        let normalized = url_normalizer::normalize(&url_str).await?;

        // Find a matching parser
        let parser = parsers
            .iter()
            .find(|p| p.can_handle(&normalized.canonical));

        match parser {
            Some(p) => {
                let resolved = p.resolve(&normalized).await?;
                results.push(resolved);
            }
            None => {
                return Err(AppError::UnsupportedPlatform {
                    message: format!("No parser available for: {}", normalized.host),
                });
            }
        }
    }

    Ok(results)
}
