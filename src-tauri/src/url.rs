//! 链接清洗与平台识别：从分享文案提取链接、抖音链接归一化、平台判断。

pub fn extract_first_url(text: &str) -> Option<String> {
    let mut start: Option<usize> = None;
    let mut end: usize = 0;
    for (i, c) in text.char_indices() {
        if start.is_none() {
            if text[i..].starts_with("https://") || text[i..].starts_with("http://") {
                start = Some(i);
                end = i + c.len_utf8();
            }
        } else if is_url_char(c) {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    start.map(|s| text[s..end].to_string())
}

/// URL 内部允许出现的字符（不含空格与中文等终止符）
pub fn is_url_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '-' | '.' | '_' | '~' | ':' | '/' | '?' | '#' | '[' | ']' | '@' | '!'
                | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '=' | '%'
        )
}

/// 清洗用户输入：先提取链接，再按平台归一化
pub fn clean_url(raw: &str) -> String {
    let extracted = extract_first_url(raw).unwrap_or_else(|| raw.to_string());
    normalize_douyin_url(&extracted)
}

/// 抖音精选页链接换算成标准视频链接
/// jingxuan?modal_id=<id> -> https://www.douyin.com/video/<id>
pub fn normalize_douyin_url(url: &str) -> String {
    if url.contains("douyin.com") {
        if let Some(idx) = url.find("modal_id=") {
            let rest = &url[idx + "modal_id=".len()..];
            let end = rest.find(['&', '#']).unwrap_or(rest.len());
            let id = &rest[..end];
            if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                return format!("https://www.douyin.com/video/{}", id);
            }
        }
    }
    url.to_string()
}

/// 从 URL 推断平台名称（用于前端展示与平台特化逻辑）。
pub fn detect_platform(url: &str) -> &'static str {
    let u = url.to_lowercase();
    if u.contains("bilibili.com") || u.contains("b23.tv") {
        "哔哩哔哩"
    } else if u.contains("youtube.com") || u.contains("youtu.be") {
        "YouTube"
    } else if u.contains("douyin.com") {
        "抖音"
    } else if u.contains("xiaohongshu.com") || u.contains("xhslink.com") {
        "小红书"
    } else if u.contains("weibo.com") {
        "微博"
    } else if u.contains("v.qq.com") {
        "腾讯视频"
    } else if u.contains("iqiyi.com") {
        "爱奇艺"
    } else if u.contains("youku.com") {
        "优酷"
    } else if u.contains("douyu.com") {
        "斗鱼"
    } else if u.contains("huya.com") {
        "虎牙"
    } else if u.contains("acfun.cn") {
        "AcFun"
    } else if host_matches(&u, "twitter.com") || host_matches(&u, "x.com") {
        "Twitter / X"
    } else if u.contains("instagram.com") {
        "Instagram"
    } else if u.contains("facebook.com") || u.contains("fb.watch") {
        "Facebook"
    } else {
        "未知平台"
    }
}

/// 按「域名标签」精确匹配 host：把 URL 拆出 host 后用 '.' 切分标签，
/// 仅当 host 的末尾标签序列等于 target 的完整标签序列时才命中。
/// 这样 `x.com` 不会误匹配 `netflix.com`（后者末尾标签是 [`netflix`,`com`]）。
fn host_matches(url_lower: &str, target: &str) -> bool {
    let host = match url_lower.find("://") {
        Some(idx) => &url_lower[idx + 3..],
        None => url_lower,
    };
    let host = host.split(['/', '?', '#']).next().unwrap_or(host);
    let host_labels: Vec<&str> = host.split('.').rev().collect();
    let target_labels: Vec<&str> = target.split('.').rev().collect();
    // host 标签数比 target 还少，不可能是其后缀（如裸 "com" 不应匹配 "x.com"）
    if host_labels.len() < target_labels.len() {
        return false;
    }
    target_labels.iter().zip(host_labels.iter()).all(|(a, b)| a == b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_platform_youtube() {
        assert_eq!(detect_platform("https://www.youtube.com/watch?v=abc"), "YouTube");
        assert_eq!(detect_platform("https://youtu.be/xyz"), "YouTube");
    }

    #[test]
    fn detect_platform_bilibili() {
        assert_eq!(detect_platform("https://www.bilibili.com/video/BV1xx"), "哔哩哔哩");
        assert_eq!(detect_platform("https://b23.tv/abc"), "哔哩哔哩");
    }

    #[test]
    fn detect_platform_unknown() {
        assert_eq!(detect_platform("https://example.com/video"), "未知平台");
        assert_eq!(detect_platform(""), "未知平台");
    }

    #[test]
    fn detect_platform_case_insensitive() {
        assert_eq!(detect_platform("https://BILIBILI.COM/video"), "哔哩哔哩");
    }

    #[test]
    fn extract_first_url_from_share_text() {
        let text = "7.46 复制打开抖音 https://v.douyin.com/abc123/ 快来";
        assert_eq!(extract_first_url(text), Some("https://v.douyin.com/abc123/".into()));
    }

    #[test]
    fn clean_url_passthrough() {
        assert_eq!(clean_url("https://www.bilibili.com/video/BV1"), "https://www.bilibili.com/video/BV1");
    }

    #[test]
    fn normalize_douyin_modal() {
        assert_eq!(
            normalize_douyin_url("https://www.douyin.com/jingxuan?modal_id=123456&x=1"),
            "https://www.douyin.com/video/123456",
        );
    }

    #[test]
    fn detect_platform_twitter() {
        assert_eq!(detect_platform("https://twitter.com/foo/status/1"), "Twitter / X");
        assert_eq!(detect_platform("https://x.com/foo"), "Twitter / X");
        assert_eq!(detect_platform("https://mobile.x.com/foo"), "Twitter / X");
    }

    #[test]
    fn detect_platform_x_com_no_false_positive() {
        // netflix.com 末尾标签是 [netflix, com]，不应因包含子串 "x.com" 被误判为 Twitter/X
        assert_eq!(detect_platform("https://www.netflix.com/watch/1"), "未知平台");
        assert_eq!(detect_platform("https://xvideo.com/foo"), "未知平台");
    }
}
