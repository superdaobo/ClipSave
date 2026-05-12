use tauri::command;
use crate::error::AppError;
use crate::models::ResolvedMedia;
use crate::parser::link_extractor;
use crate::parser::registry::resolve_urls;

/// Parse share text or URLs, extract links, normalize, and resolve media.
#[command]
pub async fn parse_links(input: String) -> Result<Vec<ResolvedMedia>, AppError> {
    if input.trim().is_empty() {
        return Err(AppError::InvalidInput {
            message: "Input text is empty".to_string(),
        });
    }

    let urls = link_extractor::extract_urls(&input);
    if urls.is_empty() {
        return Ok(vec![]);
    }

    let results = resolve_urls(urls).await?;
    Ok(results)
}
