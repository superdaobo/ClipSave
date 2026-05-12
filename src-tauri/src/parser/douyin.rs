use async_trait::async_trait;
use regex::Regex;
use scraper::{Html, Selector};

use crate::error::AppError;
use crate::models::{MediaItem, MediaType, Platform, ResolvedMedia};
use crate::parser::common::{NormalizedUrl, Parser};
use crate::parser::fetcher;

/// Douyin platform parser.
/// Handles: douyin.com, www.douyin.com, iesdouyin.com, www.iesdouyin.com, v.douyin.com
///
/// Douyin uses DASH streaming: separate video and audio tracks.
/// This parser identifies muxed (combined) streams when available,
/// or pairs video+audio streams for ffmpeg merging.
pub struct DouyinParser;

impl DouyinParser {
    pub fn new() -> Self {
        Self
    }

    fn is_douyin_host(host: &str) -> bool {
        matches!(
            host,
            "douyin.com"
                | "www.douyin.com"
                | "iesdouyin.com"
                | "www.iesdouyin.com"
                | "v.douyin.com"
        )
    }
}

#[async_trait]
impl Parser for DouyinParser {
    fn can_handle(&self, url: &str) -> bool {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                return Self::is_douyin_host(host);
            }
        }
        false
    }

    async fn resolve(&self, url: &NormalizedUrl) -> Result<ResolvedMedia, AppError> {
        let html_content = fetcher::fetch_page(&url.canonical).await?;

        if is_truly_restricted(&html_content) {
            return Err(AppError::RestrictedContent {
                message: "This Douyin content requires login or is private".to_string(),
            });
        }

        let document = Html::parse_document(&html_content);
        let mut title = extract_page_title(&document);
        let mut author = extract_author(&document);

        // Also try to extract title from RENDER_DATA (more reliable for /jingxuan pages)
        if title.is_none() {
            title = extract_title_from_render_data(&html_content);
        }
        if author.is_none() {
            author = extract_author_from_render_data(&html_content);
        }

        // Strategy 1: Extract from RENDER_DATA (primary method)
        let mut media_items = extract_from_render_data(&html_content);

        // Strategy 2: OG tags fallback
        if media_items.is_empty() {
            media_items = extract_from_og_tags(&document);
        }

        // Strategy 3: JSON-LD fallback
        if media_items.is_empty() {
            media_items = extract_from_json_ld(&document);
        }

        if media_items.is_empty() {
            return Err(AppError::ParseFailed {
                message: "Could not find public media URLs on this Douyin page. The page structure may have changed.".to_string(),
                platform_hint: Some("douyin".to_string()),
            });
        }

        Ok(ResolvedMedia {
            platform: Platform::Douyin,
            source_url: url.original.clone(),
            canonical_url: url.canonical.clone(),
            title,
            author,
            media_items,
            cover: extract_og_content(&document, "og:image"),
            created_at: None,
        })
    }
}

/// A parsed stream from RENDER_DATA.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ParsedStream {
    url: String,
    stream_type: StreamType,
    bitrate: u32,
    quality_score: u32, // from qs= param
    content_path: String, // path after domain, for dedup
}

#[derive(Debug, Clone, PartialEq)]
enum StreamType {
    VideoOnly,  // media-video-avc1 (DASH video, no audio)
    AudioOnly,  // media-audio-und-mp4a (DASH audio)
    Muxed,      // Combined video+audio (older format, tos-cn-vd paths)
}

fn is_truly_restricted(html: &str) -> bool {
    let has_private_indicator = html.contains("作品已被删除")
        || html.contains("该作品已不存在")
        || html.contains("作品不存在")
        || html.contains("该视频已被删除");

    let has_video_data = html.contains("douyinvod.com")
        || html.contains("play_addr")
        || html.contains("playAddr");

    has_private_indicator && !has_video_data
}

/// Extract and classify all streams from RENDER_DATA.
/// Douyin uses DASH: separate video-only and audio-only streams.
/// This function:
/// 1. Extracts all douyinvod.com URLs
/// 2. Classifies them as video-only, audio-only, or muxed
/// 3. Deduplicates CDN mirrors (same content on different CDN nodes)
/// 4. Groups by quality level
/// 5. Returns one MediaItem per quality with video_url and audio_url paired
fn extract_from_render_data(html: &str) -> Vec<MediaItem> {
    let render_data = extract_render_data_content(html);
    if render_data.is_empty() {
        return vec![];
    }

    let decoded = match percent_encoding::percent_decode_str(&render_data).decode_utf8() {
        Ok(d) => d.to_string(),
        Err(_) => return vec![],
    };

    // Match all douyinvod.com URLs
    let vod_regex = Regex::new(
        r#"https?://[^"]*douyinvod\.com[^"]*"#
    ).unwrap();

    let mut all_streams: Vec<ParsedStream> = Vec::new();

    for cap in vod_regex.find_iter(&decoded) {
        let url = cap.as_str().to_string();

        // Skip non-video URLs (images, etc.)
        if !url.contains("mime_type=video_mp4") && !url.contains("/video/") {
            continue;
        }

        let stream_type = classify_stream(&url);
        let bitrate = extract_param_u32(&url, "br").unwrap_or(0);
        let quality_score = extract_param_u32(&url, "qs").unwrap_or(0);
        let content_path = extract_content_path(&url);

        all_streams.push(ParsedStream {
            url,
            stream_type,
            bitrate,
            quality_score,
            content_path,
        });
    }

    // Deduplicate: keep only one URL per unique content path
    let mut deduped_streams: Vec<ParsedStream> = Vec::new();
    for stream in &all_streams {
        if !deduped_streams.iter().any(|s| s.content_path == stream.content_path) {
            deduped_streams.push(stream.clone());
        }
    }

    // Separate by type
    let video_streams: Vec<&ParsedStream> = deduped_streams
        .iter()
        .filter(|s| s.stream_type == StreamType::VideoOnly)
        .collect();
    let audio_streams: Vec<&ParsedStream> = deduped_streams
        .iter()
        .filter(|s| s.stream_type == StreamType::AudioOnly)
        .collect();
    let muxed_streams: Vec<&ParsedStream> = deduped_streams
        .iter()
        .filter(|s| s.stream_type == StreamType::Muxed)
        .collect();

    let mut items: Vec<MediaItem> = Vec::new();

    // Prefer muxed streams (video+audio combined, no merging needed)
    if !muxed_streams.is_empty() {
        let mut sorted_muxed: Vec<&&ParsedStream> = muxed_streams.iter().collect();
        sorted_muxed.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));

        for stream in sorted_muxed {
            let quality_label = bitrate_to_quality_label(stream.bitrate);
            items.push(MediaItem {
                media_type: MediaType::Video,
                url: stream.url.clone(),
                filename_hint: Some(format!("{}_muxed", quality_label)),
                mime_type: Some("video/mp4".to_string()),
                size: None,
                bitrate: Some(stream.bitrate),
                quality_label: Some(format!("{} (含音频)", quality_label)),
            });
        }
    }

    // If no muxed streams, pair video+audio by quality score
    if items.is_empty() && !video_streams.is_empty() {
        // Sort video streams by bitrate descending
        let mut sorted_video: Vec<&&ParsedStream> = video_streams.iter().collect();
        sorted_video.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));

        // Find the best audio stream (highest bitrate)
        let best_audio = audio_streams.iter().max_by_key(|s| s.bitrate);
        let audio_url = best_audio.map(|a| a.url.as_str()).unwrap_or("");

        for video_stream in sorted_video {
            let quality_label = bitrate_to_quality_label(video_stream.bitrate);

            // Encode both video and audio URLs in a special format
            // The downloader will detect this and merge them
            let combined_url = if !audio_url.is_empty() {
                format!("DASH_MERGE||{}||{}", video_stream.url, audio_url)
            } else {
                // No audio available, just use video stream
                video_stream.url.clone()
            };

            let label = if !audio_url.is_empty() {
                format!("{} (视频+音频需合并)", quality_label)
            } else {
                format!("{} (仅视频)", quality_label)
            };

            items.push(MediaItem {
                media_type: MediaType::Video,
                url: combined_url,
                filename_hint: Some(quality_label.clone()),
                mime_type: Some("video/mp4".to_string()),
                size: None,
                bitrate: Some(video_stream.bitrate),
                quality_label: Some(label),
            });
        }
    }

    // Limit to reasonable number of quality options (top 4)
    if items.len() > 4 {
        items.truncate(4);
    }

    items
}

/// Classify a stream URL as video-only, audio-only, or muxed.
/// Priority: check for audio markers first, then check path type.
fn classify_stream(url: &str) -> StreamType {
    // Audio streams always have "media-audio" in the path
    if url.contains("media-audio-und-mp4a") || url.contains("media-audio") {
        return StreamType::AudioOnly;
    }

    // Check if it's a muxed stream (tos-cn-vd = video-download = combined)
    // Note: muxed streams may also contain "media-video-avc1" in newer formats
    if url.contains("tos-cn-vd") {
        return StreamType::Muxed;
    }

    // DASH video-only streams use tos-cn-ve (video-encode) paths
    if url.contains("media-video-avc1") || url.contains("media-video-hev1") {
        return StreamType::VideoOnly;
    }

    if url.contains("tos-cn-ve") {
        return StreamType::VideoOnly;
    }

    // Default: assume muxed if we can't determine
    StreamType::Muxed
}

/// Extract the content-identifying path portion for deduplication.
/// Different CDN nodes serve the same content at different URLs.
/// The unique part is the path between the hash and the filename.
fn extract_content_path(url: &str) -> String {
    // Pattern: https://v26-web.douyinvod.com/{hash}/{expiry}/video/tos/cn/{path}/media-video-avc1/...
    // The unique identifier is the segment after tos/cn/ (the content path)
    if let Some(tos_idx) = url.find("/tos/") {
        let after_tos = &url[tos_idx..];
        // Take up to the query string
        let path = after_tos.split('?').next().unwrap_or(after_tos);
        return path.to_string();
    }

    // Fallback: use everything after the domain up to query
    url.split("douyinvod.com")
        .nth(1)
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .to_string()
}

/// Extract a u32 parameter value from a URL query string.
fn extract_param_u32(url: &str, param: &str) -> Option<u32> {
    let pattern = format!(r"[?&]{}=(\d+)", regex::escape(param));
    let re = Regex::new(&pattern).ok()?;
    re.captures(url)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

/// Convert bitrate (kbps) to a human-readable quality label.
fn bitrate_to_quality_label(bitrate: u32) -> String {
    if bitrate >= 4000 {
        "1080p高清".to_string()
    } else if bitrate >= 2000 {
        "720p高清".to_string()
    } else if bitrate >= 1200 {
        "480p标清".to_string()
    } else if bitrate >= 600 {
        "360p流畅".to_string()
    } else if bitrate > 0 {
        format!("{}kbps", bitrate)
    } else {
        "默认".to_string()
    }
}

fn extract_render_data_content(html: &str) -> String {
    let start_markers = [
        r#"<script id="RENDER_DATA" type="application/json">"#,
        r#"<script id="RENDER_DATA">"#,
    ];

    for marker in start_markers {
        if let Some(start_idx) = html.find(marker) {
            let content_start = start_idx + marker.len();
            if let Some(end_idx) = html[content_start..].find("</script>") {
                return html[content_start..content_start + end_idx].to_string();
            }
        }
    }

    String::new()
}

/// Extract video URLs from douyinvod.com patterns anywhere in the page source (fallback).
#[allow(dead_code)]
fn extract_vod_urls_from_page(html: &str) -> Vec<MediaItem> {
    // Only look for muxed streams in fallback mode
    let vod_regex = Regex::new(
        r#"https?://[^\s"'<>]*douyinvod\.com[^\s"'<>]*tos-cn-vd[^\s"'<>]*"#
    ).unwrap();

    let mut items = Vec::new();

    if let Some(first_match) = vod_regex.find(html) {
        let video_url = first_match.as_str().to_string();
        let bitrate = extract_param_u32(&video_url, "br");
        let quality_label = bitrate.map(|b| bitrate_to_quality_label(b));
        items.push(MediaItem {
            media_type: MediaType::Video,
            url: video_url,
            filename_hint: quality_label.clone(),
            mime_type: Some("video/mp4".to_string()),
            size: None,
            bitrate,
            quality_label,
        });
    }

    items
}

fn extract_from_og_tags(document: &Html) -> Vec<MediaItem> {
    let mut items = Vec::new();

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

    if let Some(video_url) = extract_og_content(document, "og:video:url") {
        if !items.iter().any(|i| i.url == video_url) {
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
    }

    items
}

fn extract_from_json_ld(document: &Html) -> Vec<MediaItem> {
    let script_selector = match Selector::parse("script[type=\"application/ld+json\"]") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    for script in document.select(&script_selector) {
        let text = script.text().collect::<String>();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(content_url) = json.get("contentUrl").and_then(|v| v.as_str()) {
                return vec![MediaItem {
                    media_type: MediaType::Video,
                    url: content_url.to_string(),
                    filename_hint: None,
                    mime_type: json
                        .get("encodingFormat")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    size: None,
                    bitrate: None,
                    quality_label: None,
                }];
            }
        }
    }

    vec![]
}

fn extract_page_title(document: &Html) -> Option<String> {
    extract_og_content(document, "og:title")
        .or_else(|| {
            let selector = Selector::parse("title").ok()?;
            let title = document.select(&selector).next()?.text().collect::<String>();
            let cleaned = title.trim().trim_end_matches(" - 抖音").to_string();
            if cleaned.is_empty() { None } else { Some(cleaned) }
        })
}

/// Extract video title from RENDER_DATA.
/// Looks for "desc" field which contains the video description/title.
fn extract_title_from_render_data(html: &str) -> Option<String> {
    let render_data = extract_render_data_content(html);
    if render_data.is_empty() {
        return None;
    }

    let decoded = percent_encoding::percent_decode_str(&render_data)
        .decode_utf8()
        .ok()?
        .to_string();

    // Look for "desc":"..." pattern (video description/title)
    let desc_re = Regex::new(r#""desc"\s*:\s*"([^"]{1,200})""#).ok()?;
    for cap in desc_re.captures_iter(&decoded) {
        if let Some(m) = cap.get(1) {
            let desc = m.as_str().to_string();
            // Skip empty or generic descriptions
            if !desc.is_empty() && desc != "null" && desc.len() > 2 {
                // Unescape unicode
                let unescaped = desc
                    .replace("\\u0026", "&")
                    .replace("\\n", " ")
                    .replace("\\t", " ");
                return Some(unescaped);
            }
        }
    }

    None
}

/// Extract author nickname from RENDER_DATA.
fn extract_author_from_render_data(html: &str) -> Option<String> {
    let render_data = extract_render_data_content(html);
    if render_data.is_empty() {
        return None;
    }

    let decoded = percent_encoding::percent_decode_str(&render_data)
        .decode_utf8()
        .ok()?
        .to_string();

    // Look for "nickname":"..." pattern
    let nick_re = Regex::new(r#""nickname"\s*:\s*"([^"]{1,100})""#).ok()?;
    if let Some(cap) = nick_re.captures(&decoded) {
        if let Some(m) = cap.get(1) {
            let nick = m.as_str().to_string();
            if !nick.is_empty() && nick != "null" {
                return Some(nick);
            }
        }
    }

    None
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
    fn test_can_handle_douyin_urls() {
        let parser = DouyinParser::new();
        assert!(parser.can_handle("https://www.douyin.com/video/123456"));
        assert!(parser.can_handle("https://v.douyin.com/iRNBho5m/"));
        assert!(parser.can_handle("https://www.douyin.com/jingxuan?modal_id=123"));
        assert!(!parser.can_handle("https://www.xiaohongshu.com/explore/123"));
    }

    #[test]
    fn test_classify_stream_video_only() {
        // DASH video-only: has media-video-avc1 AND tos-cn-ve path
        let url = "https://v26-web.douyinvod.com/hash/exp/video/tos/cn/tos-cn-ve-15/abc/media-video-avc1/?br=1278&mime_type=video_mp4";
        assert_eq!(classify_stream(url), StreamType::VideoOnly);
    }

    #[test]
    fn test_classify_stream_audio_only() {
        let url = "https://v26-web.douyinvod.com/hash/exp/video/tos/cn/tos-cn-ve-15/abc/media-audio-und-mp4a/?br=57&mime_type=video_mp4";
        assert_eq!(classify_stream(url), StreamType::AudioOnly);
    }

    #[test]
    fn test_classify_stream_muxed() {
        // Muxed: has tos-cn-vd path (even if it also has media-video-avc1)
        let url = "https://v26-web.douyinvod.com/hash/exp/video/tos/cn/tos-cn-vd-0026/abc/?br=1218&mime_type=video_mp4";
        assert_eq!(classify_stream(url), StreamType::Muxed);
    }

    #[test]
    fn test_classify_stream_muxed_with_media_video() {
        // Muxed: tos-cn-vd takes priority over media-video-avc1
        let url = "https://v26-web.douyinvod.com/hash/exp/video/tos/cn/tos-cn-vd-0026/abc/media-video-avc1/?br=1218&mime_type=video_mp4";
        assert_eq!(classify_stream(url), StreamType::Muxed);
    }

    #[test]
    fn test_extract_param_u32() {
        let url = "https://example.com/video?a=6383&br=1278&bt=1278&qs=15";
        assert_eq!(extract_param_u32(url, "br"), Some(1278));
        assert_eq!(extract_param_u32(url, "qs"), Some(15));
        assert_eq!(extract_param_u32(url, "missing"), None);
    }

    #[test]
    fn test_bitrate_to_quality_label() {
        assert_eq!(bitrate_to_quality_label(5000), "1080p高清");
        assert_eq!(bitrate_to_quality_label(2500), "720p高清");
        assert_eq!(bitrate_to_quality_label(1300), "480p标清");
        assert_eq!(bitrate_to_quality_label(700), "360p流畅");
    }

    #[test]
    fn test_extract_content_path() {
        let url1 = "https://v11-weba.douyinvod.com/hash1/exp/video/tos/cn/tos-cn-ve-15/abc123/media-video-avc1/?br=1278";
        let url2 = "https://v26-web.douyinvod.com/hash2/exp/video/tos/cn/tos-cn-ve-15/abc123/media-video-avc1/?br=1278";
        assert_eq!(extract_content_path(url1), extract_content_path(url2));
    }

    #[test]
    fn test_extract_content_path_different_content() {
        let url1 = "https://v26-web.douyinvod.com/h/e/video/tos/cn/tos-cn-ve-15/video_a/media-video-avc1/?br=1278";
        let url2 = "https://v26-web.douyinvod.com/h/e/video/tos/cn/tos-cn-ve-15/video_b/media-audio-und-mp4a/?br=57";
        assert_ne!(extract_content_path(url1), extract_content_path(url2));
    }

    #[test]
    fn test_is_truly_restricted() {
        assert!(!is_truly_restricted("Normal page with douyinvod.com video"));
        assert!(is_truly_restricted("作品已被删除 no video here"));
        assert!(!is_truly_restricted("作品已被删除 but douyinvod.com exists"));
    }

    #[test]
    fn test_dash_merge_url_format() {
        let video = "https://cdn.douyinvod.com/video";
        let audio = "https://cdn.douyinvod.com/audio";
        let combined = format!("DASH_MERGE||{}||{}", video, audio);
        assert!(combined.starts_with("DASH_MERGE||"));
        let parts: Vec<&str> = combined.split("||").collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "DASH_MERGE");
        assert_eq!(parts[1], video);
        assert_eq!(parts[2], audio);
    }
}
