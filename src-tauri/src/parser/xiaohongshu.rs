use async_trait::async_trait;
use regex::Regex;
use scraper::{Html, Selector};

use crate::error::AppError;
use crate::models::{MediaItem, MediaType, Platform, ResolvedMedia};
use crate::parser::common::{NormalizedUrl, Parser};

/// Xiaohongshu (小红书) platform parser.
/// Handles: xiaohongshu.com, www.xiaohongshu.com, xhslink.com
///
/// Supports URL paths: /explore/{id}, /discovery/item/{id}
/// Also handles xhslink.com short links with /o/ or /a/ path prefixes.
///
/// Extracts media from publicly accessible Xiaohongshu pages using:
/// - Open Graph meta tags (og:image for multiple images)
/// - Embedded SSR data in __INITIAL_STATE__ or similar script tags
/// - JSON-LD structured data
///
/// Does NOT use private APIs, solve captcha, or bypass xsec_token validation.
pub struct XiaohongshuParser;

impl XiaohongshuParser {
    pub fn new() -> Self {
        Self
    }

    fn is_xhs_host(host: &str) -> bool {
        matches!(
            host,
            "xiaohongshu.com" | "www.xiaohongshu.com" | "xhslink.com"
        )
    }
}

#[async_trait]
impl Parser for XiaohongshuParser {
    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                return Self::is_xhs_host(host);
            }
        }
        false
    }

    async fn resolve(&self, url: &NormalizedUrl) -> Result<ResolvedMedia, AppError> {
        // For xhslink.com, we fetch the page directly with mobile UA
        // because it renders SSR content (unlike the SPA at xiaohongshu.com)
        let (html_content, page_url) = if url.host == "xhslink.com" {
            let html = fetch_xhs_page(&url.canonical).await?;
            // Try to find the canonical xiaohongshu.com URL for metadata
            let canonical = extract_canonical_url(&html).unwrap_or_else(|| url.canonical.clone());
            (html, canonical)
        } else {
            let html = fetch_xhs_page(&url.canonical).await?;
            (html, url.canonical.clone())
        };

        // Check for truly restricted content
        if is_truly_restricted(&html_content) {
            return Err(AppError::RestrictedContent {
                message: "This Xiaohongshu content requires login or is private".to_string(),
            });
        }

        let document = Html::parse_document(&html_content);

        // Extract metadata
        let title = extract_page_title(&document);
        let author = extract_author(&document);

        // Strategy 1: Extract from og:image meta tags (works with mobile UA SSR)
        let mut media_items = extract_from_og_tags(&document);

        // Strategy 2: Extract from __INITIAL_STATE__ embedded data
        if media_items.is_empty() {
            media_items = extract_from_initial_state(&html_content);
        }

        // Strategy 3: Extract from JSON-LD
        if media_items.is_empty() {
            media_items = extract_from_json_ld(&document);
        }

        // Strategy 4: Extract image URLs from xhscdn.com patterns in page
        if media_items.is_empty() {
            media_items = extract_cdn_urls_from_page(&html_content);
        }

        if media_items.is_empty() {
            return Err(AppError::ParseFailed {
                message: "Could not find public media URLs on this Xiaohongshu page. The page structure may have changed.".to_string(),
                platform_hint: Some("xiaohongshu".to_string()),
            });
        }

        Ok(ResolvedMedia {
            platform: Platform::Xiaohongshu,
            source_url: url.original.clone(),
            canonical_url: page_url,
            title,
            author,
            media_items,
            cover: extract_og_content(&document, "og:image"),
            created_at: None,
        })
    }
}

/// Fetch a Xiaohongshu page using mobile User-Agent.
/// Mobile UA triggers server-side rendering with og:image meta tags,
/// unlike the desktop SPA which renders everything client-side.
async fn fetch_xhs_page(url: &str) -> Result<String, AppError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(15))
        // Use mobile UA to get SSR content with meta tags
        .user_agent("Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1")
        .build()
        .map_err(|e| AppError::NetworkError {
            message: format!("Failed to create client: {}", e),
        })?;

    let response = client.get(url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .send()
        .await
        .map_err(|e| AppError::NetworkError {
            message: format!("Failed to fetch Xiaohongshu page: {}", e),
        })?;

    let status = response.status();
    if status.as_u16() == 404 {
        return Err(AppError::ContentNotFound {
            message: "笔记不存在或已被删除".to_string(),
        });
    }
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(AppError::RestrictedContent {
            message: "内容需要登录或不可公开访问".to_string(),
        });
    }

    response.text().await.map_err(|e| AppError::NetworkError {
        message: format!("Failed to read response: {}", e),
    })
}

/// Extract the canonical xiaohongshu.com URL from the page.
fn extract_canonical_url(html: &str) -> Option<String> {
    let document = Html::parse_document(html);

    // Try canonical link
    let selector = Selector::parse("link[rel=\"canonical\"]").ok()?;
    if let Some(el) = document.select(&selector).next() {
        if let Some(href) = el.value().attr("href") {
            if href.contains("xiaohongshu.com") {
                return Some(href.to_string());
            }
        }
    }

    // Try og:url
    let og_selector = Selector::parse("meta[property=\"og:url\"]").ok()?;
    if let Some(el) = document.select(&og_selector).next() {
        if let Some(content) = el.value().attr("content") {
            if content.contains("xiaohongshu.com") {
                return Some(content.to_string());
            }
        }
    }

    // Try to find xiaohongshu.com/explore/ URL in the page
    let re = Regex::new(r#"https?://(?:www\.)?xiaohongshu\.com/(?:explore|discovery/item)/[a-zA-Z0-9]+"#).ok()?;
    re.find(html).map(|m| m.as_str().to_string())
}

/// Check if the page is truly restricted (not just showing a login prompt overlay).
/// Xiaohongshu shows login prompts on public pages but content is still accessible.
fn is_truly_restricted(html: &str) -> bool {
    let has_deleted = html.contains("该笔记已被删除")
        || html.contains("笔记不存在")
        || html.contains("内容已不存在");

    let has_media_data = html.contains("xhscdn.com")
        || html.contains("sns-img")
        || html.contains("og:image")
        || html.contains("imageList")
        || html.contains("image_list");

    // Only restricted if explicitly deleted AND no media data
    has_deleted && !has_media_data
}

/// Extract images/videos from __INITIAL_STATE__ or similar embedded SSR data.
fn extract_from_initial_state(html: &str) -> Vec<MediaItem> {
    let mut items = Vec::new();

    // Look for image URLs from xhscdn.com (Xiaohongshu's CDN)
    // Pattern: https://sns-img-bd.xhscdn.com/{path} or https://ci.xiaohongshu.com/{path}
    let img_re = Regex::new(
        r#"https?://(?:sns-img[^"]*\.xhscdn\.com|ci\.xiaohongshu\.com)/[^"'\s\\\)>]+"#
    ).unwrap();

    let mut seen_urls: Vec<String> = Vec::new();

    for cap in img_re.find_iter(html) {
        let img_url = cap.as_str().to_string();

        // Skip thumbnails and tiny images (usually contain /w/ or dimensions)
        if img_url.contains("/w/80") || img_url.contains("/w/40") || img_url.contains("avatar") {
            continue;
        }

        // Deduplicate
        let base_url = img_url.split('?').next().unwrap_or(&img_url).to_string();
        if seen_urls.contains(&base_url) {
            continue;
        }
        seen_urls.push(base_url);

        let media_type = if img_url.contains(".gif") || img_url.contains("format/gif") {
            MediaType::Gif
        } else if img_url.contains("video") || img_url.contains(".mp4") {
            MediaType::Video
        } else {
            MediaType::Image
        };

        items.push(MediaItem {
            media_type,
            url: img_url,
            filename_hint: Some(format!("image_{}", items.len() + 1)),
            mime_type: Some("image/jpeg".to_string()),
            size: None,
            bitrate: None,
            quality_label: None,
        });
    }

    // Also look for video URLs
    let video_re = Regex::new(
        r#"https?://(?:sns-video[^"]*\.xhscdn\.com|sns-video[^"]*\.xhscdn\.net)[^"'\s\\\)>]+"#
    ).unwrap();

    for cap in video_re.find_iter(html) {
        let video_url = cap.as_str().to_string();
        let base_url = video_url.split('?').next().unwrap_or(&video_url).to_string();
        if seen_urls.contains(&base_url) {
            continue;
        }
        seen_urls.push(base_url);

        items.push(MediaItem {
            media_type: MediaType::Video,
            url: video_url,
            filename_hint: Some(format!("video_{}", items.len() + 1)),
            mime_type: Some("video/mp4".to_string()),
            size: None,
            bitrate: None,
            quality_label: None,
        });
    }

    items
}

/// Extract from Open Graph meta tags.
fn extract_from_og_tags(document: &Html) -> Vec<MediaItem> {
    let mut items = Vec::new();

    // Video
    if let Some(video_url) = extract_og_content(document, "og:video") {
        items.push(MediaItem {
            media_type: MediaType::Video,
            url: video_url,
            filename_hint: None,
            mime_type: Some("video/mp4".to_string()),
            size: None,
            bitrate: None,
            quality_label: None,
        });
    }

    // Images (Xiaohongshu often has multiple og:image tags)
    let og_images = extract_all_og_images(document);
    for (idx, image_url) in og_images.into_iter().enumerate() {
        let media_type = if image_url.contains(".gif") || image_url.contains("format/gif") {
            MediaType::Gif
        } else {
            MediaType::Image
        };

        items.push(MediaItem {
            media_type,
            url: image_url,
            filename_hint: Some(format!("image_{}", idx + 1)),
            mime_type: Some("image/jpeg".to_string()),
            size: None,
            bitrate: None,
            quality_label: None,
        });
    }

    items
}

/// Extract from JSON-LD structured data.
fn extract_from_json_ld(document: &Html) -> Vec<MediaItem> {
    let script_selector = match Selector::parse("script[type=\"application/ld+json\"]") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    for script in document.select(&script_selector) {
        let text = script.text().collect::<String>();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            let mut items = Vec::new();

            if let Some(images) = json.get("image").and_then(|v| v.as_array()) {
                for (idx, img) in images.iter().enumerate() {
                    if let Some(url) = img.as_str() {
                        items.push(MediaItem {
                            media_type: MediaType::Image,
                            url: url.to_string(),
                            filename_hint: Some(format!("image_{}", idx + 1)),
                            mime_type: Some("image/jpeg".to_string()),
                            size: None,
                            bitrate: None,
                            quality_label: None,
                        });
                    }
                }
            }

            if let Some(video_url) = json.get("contentUrl").and_then(|v| v.as_str()) {
                items.push(MediaItem {
                    media_type: MediaType::Video,
                    url: video_url.to_string(),
                    filename_hint: None,
                    mime_type: Some("video/mp4".to_string()),
                    size: None,
                    bitrate: None,
                    quality_label: None,
                });
            }

            if !items.is_empty() {
                return items;
            }
        }
    }

    vec![]
}

/// Extract xhscdn.com image/video URLs from anywhere in the page (fallback).
fn extract_cdn_urls_from_page(html: &str) -> Vec<MediaItem> {
    let mut items = Vec::new();

    let cdn_re = Regex::new(
        r#"https?://[^"'\s\\]*xhscdn\.[^"'\s\\]*"#
    ).unwrap();

    let mut seen: Vec<String> = Vec::new();

    for cap in cdn_re.find_iter(html) {
        let url = cap.as_str().to_string();

        // Skip CSS/JS/font resources
        if url.contains(".css") || url.contains(".js") || url.contains("font") || url.contains("woff") {
            continue;
        }

        let base = url.split('?').next().unwrap_or(&url).to_string();
        if seen.contains(&base) {
            continue;
        }
        seen.push(base);

        let media_type = if url.contains("video") || url.contains(".mp4") {
            MediaType::Video
        } else {
            MediaType::Image
        };

        items.push(MediaItem {
            media_type: media_type.clone(),
            url,
            filename_hint: Some(format!("media_{}", items.len() + 1)),
            mime_type: if media_type == MediaType::Video {
                Some("video/mp4".to_string())
            } else {
                Some("image/jpeg".to_string())
            },
            size: None,
            bitrate: None,
            quality_label: None,
        });
    }

    items
}

fn extract_all_og_images(document: &Html) -> Vec<String> {
    let selector = match Selector::parse("meta[property=\"og:image\"]") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    document
        .select(&selector)
        .filter_map(|el| el.value().attr("content"))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn extract_page_title(document: &Html) -> Option<String> {
    extract_og_content(document, "og:title")
        .or_else(|| {
            let selector = Selector::parse("title").ok()?;
            let title = document.select(&selector).next()?.text().collect::<String>();
            let cleaned = title.trim().trim_end_matches(" - 小红书").to_string();
            if cleaned.is_empty() { None } else { Some(cleaned) }
        })
}

fn extract_author(document: &Html) -> Option<String> {
    extract_og_content(document, "og:author")
        .or_else(|| extract_meta_content(document, "author"))
}

fn extract_og_content(document: &Html, property: &str) -> Option<String> {
    let selector =
        Selector::parse(&format!("meta[property=\"{}\"]", property)).ok()?;
    document
        .select(&selector)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

fn extract_meta_content(document: &Html, name: &str) -> Option<String> {
    let selector = Selector::parse(&format!("meta[name=\"{}\"]", name)).ok()?;
    document
        .select(&selector)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_xhs_urls() {
        let parser = XiaohongshuParser::new();
        assert!(parser.can_handle("https://www.xiaohongshu.com/explore/abc123"));
        assert!(parser.can_handle("https://xiaohongshu.com/discovery/item/abc123"));
        assert!(parser.can_handle("https://xhslink.com/a/abc123"));
        assert!(parser.can_handle("http://xhslink.com/o/8BsnjP0zJKx"));
        assert!(!parser.can_handle("https://www.douyin.com/video/123"));
    }

    #[test]
    fn test_is_truly_restricted() {
        // Page with login prompt but has media data — NOT restricted
        assert!(!is_truly_restricted("请登录 xhscdn.com/image.jpg"));
        // Deleted note without media — restricted
        assert!(is_truly_restricted("该笔记已被删除 no media here"));
        // Normal page — not restricted
        assert!(!is_truly_restricted("Normal content with xhscdn.com images"));
    }

    #[test]
    fn test_extract_from_initial_state() {
        let html = r#"
        <script>window.__INITIAL_STATE__={"note":{"imageList":["https://sns-img-bd.xhscdn.com/abc123.jpg","https://sns-img-bd.xhscdn.com/def456.jpg"]}}</script>
        "#;
        let items = extract_from_initial_state(html);
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.media_type == MediaType::Image));
    }

    #[test]
    fn test_extract_from_og_tags() {
        let html = r#"
        <html><head>
            <meta property="og:title" content="Beautiful Photos" />
            <meta property="og:image" content="https://sns-img-bd.xhscdn.com/img1.jpg" />
            <meta property="og:image" content="https://sns-img-bd.xhscdn.com/img2.jpg" />
        </head><body></body></html>
        "#;
        let document = Html::parse_document(html);
        let items = extract_from_og_tags(&document);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_extract_cdn_urls() {
        let html = r#"something "https://sns-img-bd.xhscdn.com/photo1.jpg?imageView" and "https://sns-img-bd.xhscdn.com/photo2.jpg" end"#;
        let items = extract_cdn_urls_from_page(html);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_skip_css_js_resources() {
        let html = r#""https://fe-static.xhscdn.com/style.css" "https://fe-static.xhscdn.com/app.js" "https://sns-img-bd.xhscdn.com/real_image.jpg""#;
        let items = extract_cdn_urls_from_page(html);
        assert_eq!(items.len(), 1);
        assert!(items[0].url.contains("real_image"));
    }
}
