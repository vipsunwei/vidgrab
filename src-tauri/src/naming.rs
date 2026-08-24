//! 输出文件名工具：清洗、扩展名剥离、质量标签拼接。
use std::path::PathBuf;

pub fn strip_extension(name: &str) -> String {
    if let Some(pos) = name.rfind('.') {
        if pos == 0 {
            return name.to_string(); // 以点开头（隐藏文件/无主名），整体保留
        }
        let ext = name[pos + 1..].to_ascii_lowercase();
        // 末尾空点（如 "foo."）或常见音视频扩展名都视为扩展名，去掉一层
        if ext.is_empty()
            || matches!(
                ext.as_str(),
                "mp4" | "webm" | "mkv" | "mov" | "avi" | "flv" | "m4v" | "mpg" | "mpeg" | "ts"
                    | "3gp" | "wmv" | "ogv" | "mp3" | "m4a" | "aac" | "wav" | "opus" | "mka"
            )
        {
            return name[..pos].to_string();
        }
    }
    name.to_string()
}

/// 计算输出文件名：自定义名优先（去扩展名），否则用视频标题。
/// 返回 (展示用基础名, sanitize + 截断后的安全文件名前缀)
pub fn compute_titles(title: &str, output_name: Option<&str>) -> (String, String) {
    let base_title = match output_name {
        Some(name) if !name.trim().is_empty() => strip_extension(name.trim()).to_string(),
        _ => title.to_string(),
    };
    let safe_title = sanitize_filename(&base_title);
    (base_title, safe_title)
}

/// 拼接质量标签到安全文件名：`{safe} [{tag}]`；tag 为空则原样返回。
/// 同一视频的不同清晰度/音质因此落在不同文件名上，可并行且互不覆盖。
pub fn with_quality_tag(safe_title: &str, quality_tag: Option<&str>) -> String {
    match quality_tag.map(str::trim).filter(|s| !s.is_empty()) {
        Some(tag) => format!("{safe_title} [{}]", sanitize_filename(tag)),
        None => safe_title.to_string(),
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Windows 路径上限 260 字符，目录前缀 + 扩展名约占 40，
    // 标题控制在 150 字符内，避免 .part 临时文件因路径过长写不进去
    let max_len = 150;
    if sanitized.chars().count() > max_len {
        sanitized.chars().take(max_len).collect()
    } else {
        sanitized
    }
}


pub fn find_latest_in_dir(dir: &PathBuf, title: &str, suffix: &str) -> Result<String, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}", e))?;

    // 用前缀匹配而非 contains：避免同系列标题互含时错拿其他任务的文件
    let mut candidates: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let binding = e.file_name();
            let name = binding.to_string_lossy();
            name.starts_with(title) && (suffix.is_empty() || name.contains(suffix))
        })
        .collect();

    candidates.sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));

    candidates
        .first()
        .map(|e| e.path().display().to_string())
        .ok_or_else(|| format!("找不到下载文件: {}{}", title, suffix))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_normal() {
        assert_eq!(sanitize_filename("正常标题"), "正常标题");
        assert_eq!(sanitize_filename("video-file_2026.mp4"), "video-file_2026.mp4");
    }

    #[test]
    fn strip_extension_removes_trailing_ext() {
        assert_eq!(strip_extension("my-video.mp4"), "my-video");
        assert_eq!(strip_extension("my-video.webm"), "my-video");
        assert_eq!(strip_extension("my.video.2024.mp4"), "my.video.2024");
        assert_eq!(strip_extension("no_ext_name"), "no_ext_name");
        assert_eq!(strip_extension("trailing-dot."), "trailing-dot");
    }

    #[test]
    fn strip_extension_keeps_hidden_prefix() {
        // 隐藏文件 .gitignore 没有中间点，整体当作名保留
        assert_eq!(strip_extension(".gitignore"), ".gitignore");
    }

    #[test]
    fn sanitize_filename_replaces_special_chars() {
        assert_eq!(sanitize_filename("title:with*special?chars"), "title_with_special_chars");
        assert_eq!(sanitize_filename("a/b:c|d<e>f"), "a_b_c_d_e_f");
    }

    #[test]
    fn sanitize_filename_emoji_handled() {
        // emoji 不是 ascii alphanumeric，替换为下划线（每个 emoji → 1 个 _）
        assert_eq!(sanitize_filename("视频🔥"), "视频_");
        assert_eq!(sanitize_filename("🎵 Music 🎵"), "_ Music _");
    }

    #[test]
    fn sanitize_filename_empty() {
        assert_eq!(sanitize_filename(""), "");
    }

    #[test]
    fn sanitize_filename_truncates_long_title() {
        let long = "x".repeat(500);
        let out = sanitize_filename(&long);
        assert!(out.chars().count() <= 150);

        // 含特殊字符的长标题同样截断，且不会因路径过长写不进去
        let title = "25K views · 646 reactions _ As fishing guides here in Alaska, we get to share some pretty incredible moments with clients—but watching a Kodiak brown bear chase down a red salmon right in front of us was something special. ".repeat(3);
        let out2 = sanitize_filename(&title);
        assert!(out2.chars().count() <= 150);
    }
}
