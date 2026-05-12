use regex::Regex;

/// Maximum number of URLs to extract from a single submission.
const MAX_BATCH_SIZE: usize = 20;

/// Extract all HTTP/HTTPS URLs from arbitrary text.
/// Returns URLs in left-to-right order of appearance, limited to MAX_BATCH_SIZE.
///
/// Supports extracting both short links (v.douyin.com/xxx, xhslink.com/xxx)
/// and long links (www.douyin.com/video/xxx, www.xiaohongshu.com/explore/xxx).
pub fn extract_urls(text: &str) -> Vec<String> {
    let url_regex = Regex::new(
        r#"https?://[^\s<>\[\]\(\)\{\}"'，。！？、；：\u{201c}\u{201d}\u{2018}\u{2019}【】《》\x00-\x1f]+"#
    ).expect("URL regex should compile");

    let urls: Vec<String> = url_regex
        .find_iter(text)
        .map(|m| {
            let url = m.as_str().to_string();
            // Trim trailing punctuation that might have been captured
            url.trim_end_matches(|c: char| matches!(c, '.' | ',' | ';' | ')' | ']' | '}' | '>' | '/' ))
                .to_string()
        })
        .filter(|url| !url.is_empty())
        .collect();

    // Enforce batch limit: return at most MAX_BATCH_SIZE URLs
    urls.into_iter().take(MAX_BATCH_SIZE).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_douyin_share_text() {
        let text = "复制这条链接，打开抖音看看…… https://v.douyin.com/iRNBho5m/ 7@5.com";
        let urls = extract_urls(text);
        assert_eq!(urls.len(), 1);
        assert!(urls[0].contains("v.douyin.com"));
    }

    #[test]
    fn test_extract_from_xiaohongshu_share_text() {
        let text = "小红书笔记分享 https://xhslink.com/a/abc123 快来看看";
        let urls = extract_urls(text);
        assert_eq!(urls.len(), 1);
        assert!(urls[0].contains("xhslink.com"));
    }

    #[test]
    fn test_extract_multiple_urls() {
        let text = "Link 1: https://v.douyin.com/abc and link 2: https://xhslink.com/def";
        let urls = extract_urls(text);
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_extract_no_urls() {
        let text = "This is just plain text without any links";
        let urls = extract_urls(text);
        assert!(urls.is_empty());
    }

    #[test]
    fn test_extract_long_url() {
        let text = "Check this: https://www.xiaohongshu.com/explore/abc123def456";
        let urls = extract_urls(text);
        assert_eq!(urls.len(), 1);
        assert!(urls[0].contains("xiaohongshu.com/explore"));
    }

    #[test]
    fn test_batch_limit_enforcement() {
        let mut text = String::new();
        for i in 0..25 {
            text.push_str(&format!("https://example.com/{} ", i));
        }
        let urls = extract_urls(&text);
        assert_eq!(urls.len(), MAX_BATCH_SIZE);
    }

    #[test]
    fn test_empty_input() {
        assert!(extract_urls("").is_empty());
        assert!(extract_urls("   ").is_empty());
    }

    #[test]
    fn test_preserves_order() {
        let text = "https://first.com https://second.com https://third.com";
        let urls = extract_urls(text);
        assert_eq!(urls.len(), 3);
        assert!(urls[0].contains("first"));
        assert!(urls[1].contains("second"));
        assert!(urls[2].contains("third"));
    }

    #[test]
    fn test_handles_chinese_punctuation() {
        let text = "看看这个视频https://v.douyin.com/test123，很有趣！";
        let urls = extract_urls(text);
        assert_eq!(urls.len(), 1);
        assert!(urls[0].contains("v.douyin.com/test123"));
    }
}
