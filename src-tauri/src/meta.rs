//! yt-dlp JSON 解析与缩略图处理：结果结构、字段过滤、封面转 data URI。
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::url::detect_platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtDlpJson {
    pub title: String,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    #[serde(rename = "duration_string")]
    pub duration_str: Option<String>,
    pub formats: Vec<YtDlpFormat>,
}

/// 单个格式条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtDlpFormat {
    pub format_id: String,
    pub ext: String,
    pub resolution: Option<String>,
    #[serde(rename = "filesize")]
    pub filesize: Option<u64>,
    #[serde(rename = "filesize_approx")]
    pub filesize_approx: Option<u64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    #[serde(rename = "format_note")]
    pub format_note: Option<String>,
    /// 音频码率（kbps）
    pub abr: Option<f64>,
    /// 音频采样率（Hz）
    pub asr: Option<u32>,
    /// 音频声道数
    pub audio_channels: Option<u8>,
}

/// 解析后返回给前端的格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFormat {
    pub format_id: String,
    pub ext: String,
    pub resolution: String,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub format_note: String,
    /// 是否有视频流（空字符串/none 表示无）
    pub vcodec: String,
    /// 是否有音频流（空字符串/none 表示无）
    pub acodec: String,
    /// 是否为纯音频格式
    pub is_audio_only: bool,
    /// 音频码率（kbps）
    pub abr: Option<f64>,
    /// 音频采样率（Hz）
    pub asr: Option<u32>,
    /// 音频声道数
    pub audio_channels: Option<u8>,
}

/// 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub thumbnail: String,
    pub uploader: String,
    pub duration: u32,
    pub platform: String,
    pub formats: Vec<VideoFormat>,
}

/// 从 yt-dlp 原始 JSON 转换为 VideoInfo
pub fn has_codec(codec: &Option<String>) -> bool {
    codec
        .as_deref()
        .map(|c| !c.is_empty() && c != "none")
        .unwrap_or(false)
}

pub fn parse_video_info(raw: &YtDlpJson, platform: &str) -> VideoInfo {
    // 计算时长（秒）
    let duration = raw.duration_str.as_ref().and_then(|s| parse_duration(s)).unwrap_or(0);

    // 缩略图 URL 统一升级为 HTTPS，避免 WebView mixed-content 阻止 HTTP 图片
    let thumbnail = raw
        .thumbnail
        .as_deref()
        .map(|u| u.replacen("http://", "https://", 1))
        .unwrap_or_default();

    // 过滤 + 映射：只要真实有视频或音频流，剔除 storyboard/none
    let formats: Vec<VideoFormat> = raw
        .formats
        .iter()
        .filter(|f| {
            let note = f.format_note.as_deref().unwrap_or("").to_lowercase();
            if note.contains("storyboard") {
                return false;
            }
            has_codec(&f.vcodec) || has_codec(&f.acodec)
        })
        .map(|f| {
            let has_video = has_codec(&f.vcodec);
            let has_audio = has_codec(&f.acodec);
            let is_audio_only = !has_video && has_audio;
            VideoFormat {
                format_id: f.format_id.clone(),
                ext: f.ext.clone(),
                resolution: f.resolution.clone().unwrap_or_else(|| {
                    if is_audio_only {
                        String::from("audio only")
                    } else {
                        String::from("unknown")
                    }
                }),
                filesize: f.filesize,
                filesize_approx: f.filesize_approx,
                format_note: f.format_note.clone().unwrap_or_default(),
                vcodec: f.vcodec.clone().unwrap_or_default(),
                acodec: f.acodec.clone().unwrap_or_default(),
                is_audio_only,
                abr: f.abr,
                asr: f.asr,
                audio_channels: f.audio_channels,
            }
        })
        .collect();

    VideoInfo {
        title: raw.title.clone(),
        thumbnail,
        uploader: raw.uploader.clone().unwrap_or_else(|| String::from("未知作者")),
        duration,
        platform: platform.to_string(),
        formats,
    }
}

/// 解析 yt-dlp --dump-json 输出的 JSON 字符串为 VideoInfo
///
/// 纯函数：输入 JSON 字符串，输出 VideoInfo 或错误
pub fn parse_ytdlp_output(json_str: &str, url: &str) -> Result<VideoInfo, String> {
    let raw: YtDlpJson = serde_json::from_str(json_str)
        .map_err(|e| format!("yt-dlp 输出 JSON 解析失败: {}", e))?;
    let platform = detect_platform(url);
    Ok(parse_video_info(&raw, platform))
}

/// 把缩略图下载成 base64 data URI，规避 WebView 的 mixed-content / 防盗链 / CSP 限制
///
/// 失败的场合返回空字符串，调用方回退到原始 URL（已升级为 HTTPS）。
pub async fn fetch_thumbnail_data_uri(thumbnail: &str) -> String {
    if thumbnail.is_empty() {
        return String::new();
    }
    // 统一走 HTTPS
    let url = thumbnail.replacen("http://", "https://", 1);
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return String::new(),
    };
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(_) => return String::new(),
    };
    // 按文件头猜 MIME，猜不到默认 image/jpeg
    let mime = guess_mime_from_bytes(&bytes);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    format!("data:{};base64,{}", mime, b64)
}

/// 根据文件头判断图片 MIME 类型
pub fn guess_mime_from_bytes(bytes: &[u8]) -> &'static str {
    match bytes {
        b if b.len() >= 3 && &b[0..3] == b"\xff\xd8\xff" => "image/jpeg",
        b if b.len() >= 8 && &b[0..8] == b"\x89PNG\r\n\x1a\n" => "image/png",
        b if b.len() >= 4 && &b[0..4] == b"RIFF" => "image/webp",
        b if b.len() >= 3 && &b[0..3] == b"GIF" => "image/gif",
        b if b.len() >= 4 && &b[0..4] == b"%PDF" => "application/pdf",
        _ => "image/jpeg",
    }
}

/// 解析 "HH:MM:SS" 或 "MM:SS" 时长字符串为秒数。
pub fn parse_duration(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            let m: u32 = parts[0].parse().ok()?;
            let s: u32 = parts[1].parse().ok()?;
            Some(m * 60 + s)
        }
        3 => {
            let h: u32 = parts[0].parse().ok()?;
            let m: u32 = parts[1].parse().ok()?;
            let s: u32 = parts[2].parse().ok()?;
            Some(h * 3600 + m * 60 + s)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_raw_json() -> YtDlpJson {
        YtDlpJson {
            title: String::from("测试视频标题"),
            thumbnail: Some(String::from("https://example.com/thumb.jpg")),
            uploader: Some(String::from("测试UP主")),
            duration_str: Some(String::from("10:30")),
            formats: vec![
                YtDlpFormat {
                    format_id: String::from("136"),
                    ext: String::from("mp4"),
                    resolution: Some(String::from("1280x720")),
                    filesize: Some(420_000_000),
                    vcodec: Some(String::from("avc1")),
                    acodec: Some(String::from("mp4a")),
                    format_note: Some(String::from("HD")),
                    filesize_approx: None,
                    abr: None,
                    asr: None,
                    audio_channels: None,
                },
                // 纯音频格式
                YtDlpFormat {
                    format_id: String::from("140"),
                    ext: String::from("m4a"),
                    resolution: None,
                    filesize: Some(45_000_000),
                    vcodec: None,
                    acodec: Some(String::from("mp4a")),
                    format_note: Some(String::from("audio")),
                    filesize_approx: None,
                    abr: None,
                    asr: None,
                    audio_channels: None,
                },
                // 纯视频（无音频轨道，不常见但存在）
                YtDlpFormat {
                    format_id: String::from("999"),
                    ext: String::from("mp4"),
                    resolution: Some(String::from("640x360")),
                    filesize: Some(80_000_000),
                    vcodec: Some(String::from("avc1")),
                    acodec: None,
                    format_note: None,
                    filesize_approx: None,
                    abr: None,
                    asr: None,
                    audio_channels: None,
                },
            ],
        }
    }

    #[test]
    fn parse_duration_invalid() {
        assert_eq!(parse_duration("not a time"), None);
        assert_eq!(parse_duration(""), None);
    }

    #[test]
    fn parse_duration_mm_ss() {
        assert_eq!(parse_duration("03:45"), Some(225));
    }

    #[test]
    fn parse_duration_hh_mm_ss() {
        assert_eq!(parse_duration("1:23:45"), Some(5025));
    }

    #[test]
    fn parse_video_info_basic() {
        let raw = make_raw_json();
        let info = parse_video_info(&raw, "YouTube");

        assert_eq!(info.title, "测试视频标题");
        assert_eq!(info.thumbnail, "https://example.com/thumb.jpg");
        assert_eq!(info.uploader, "测试UP主");
        assert_eq!(info.duration, 630); // 10*60+30
        assert_eq!(info.platform, "YouTube");
    }

    #[test]
    fn parse_video_info_filters_formats() {
        let raw = make_raw_json();
        let info = parse_video_info(&raw, "YouTube");

        // 3 个格式入参，3 个都保留（vcodec+acodec / 仅acodec / 仅vcodec）
        assert_eq!(info.formats.len(), 3);

        // 确认 is_audio_only 标记
        let audio_fmt = info.formats.iter().find(|f| f.format_id == "140").unwrap();
        assert!(audio_fmt.is_audio_only);
        assert_eq!(audio_fmt.resolution, "audio only");

        let video_fmt = info.formats.iter().find(|f| f.format_id == "136").unwrap();
        assert!(!video_fmt.is_audio_only);
        assert_eq!(video_fmt.ext, "mp4");
        assert_eq!(video_fmt.filesize, Some(420_000_000));
    }

    #[test]
    fn parse_video_info_missing_optional_fields() {
        let raw = YtDlpJson {
            title: String::from("无封面视频"),
            thumbnail: None,
            uploader: None,
            duration_str: None,
            formats: vec![],
        };
        let info = parse_video_info(&raw, "抖音");

        assert_eq!(info.title, "无封面视频");
        assert_eq!(info.thumbnail, "");
        assert_eq!(info.uploader, "未知作者");
        assert_eq!(info.duration, 0);
        assert!(info.formats.is_empty());
    }

    #[test]
    fn parse_video_info_resolution_fallback() {
        // 纯音频格式没有 resolution，应该填充 "audio only"
        let raw = YtDlpJson {
            title: String::from("音频"),
            thumbnail: None,
            uploader: None,
            duration_str: None,
            formats: vec![YtDlpFormat {
                format_id: String::from("140"),
                ext: String::from("m4a"),
                resolution: None,
                filesize: None,
                vcodec: None,
                acodec: Some(String::from("mp4a")),
                    format_note: None,
                    filesize_approx: None,
                    abr: None,
                    asr: None,
                    audio_channels: None,
            }],
        };
        let info = parse_video_info(&raw, "YouTube");
        assert_eq!(info.formats[0].resolution, "audio only");
    }

    // parse_ytdlp_output

    #[test]
    fn parse_ytdlp_output_valid() {
        // 真实 yt-dlp --dump-json 输出片段（从 YouTube 抓取）
        let json = r#"{
            "title": "2026年5月㷧紅華語神曲🔥",
            "thumbnail": "https://i.ytimg.com/vi/3Va-3carIcA/maxresdefault.jpg",
            "uploader": "音樂清單頻道",
            "duration_string": "1:02:34",
            "formats": [
                {
                    "format_id": "136",
                    "ext": "mp4",
                    "resolution": "1280x720",
                    "filesize": 836540000,
                    "vcodec": "avc1.4d401f",
                    "acodec": "mp4a.40.2",
                    "format_note": "720p"
                },
                {
                    "format_id": "140",
                    "ext": "m4a",
                    "vcodec": null,
                    "acodec": "mp4a.40.2",
                    "format_note": "audio only"
                }
            ]
        }"#;

        let info = parse_ytdlp_output(json, "https://www.youtube.com/watch?v=3Va-3carIcA")
            .expect("should parse");

        assert_eq!(info.title, "2026年5月㷧紅華語神曲🔥");
        assert_eq!(info.thumbnail, "https://i.ytimg.com/vi/3Va-3carIcA/maxresdefault.jpg");
        assert_eq!(info.uploader, "音樂清單頻道");
        assert_eq!(info.duration, 3754); // 1*3600+2*60+34
        assert_eq!(info.platform, "YouTube");
        assert_eq!(info.formats.len(), 2);
    }

    #[test]
    fn parse_ytdlp_output_invalid_json() {
        let json = "{ invalid json }";
        let result = parse_ytdlp_output(json, "https://example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON 解析失败"));
    }

    #[test]
    fn parse_ytdlp_output_missing_required_fields() {
        // 缺 title（必填字段）
        let json = r#"{
            "duration_string": "10:00",
            "formats": []
        }"#;
        let result = parse_ytdlp_output(json, "https://example.com");
        assert!(result.is_err());
    }

    #[test]
    fn parse_ytdlp_output_detects_platform_from_url() {
        let json = r#"{
            "title": "B站视频",
            "duration_string": "5:30",
            "formats": []
        }"#;

        let info = parse_ytdlp_output(json, "https://www.bilibili.com/video/BV1xx")
            .expect("should parse");
        assert_eq!(info.platform, "哔哩哔哩");
    }

    // 集成测试（需网络，默认忽略，跑：cargo test -- --ignored）

}
