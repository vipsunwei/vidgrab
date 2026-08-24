//! 下载进度：事件负载、回传与 yt-dlp/ffmpeg 进度行解析。
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub task_id: String,
    pub stage: String,
    pub progress: f64,
    pub speed: String,
    pub eta: String,
    pub filename: String,
}

#[derive(Clone, Serialize)]
pub struct DownloadDonePayload {
    pub task_id: String,
    pub output_path: String,
    pub title: String,
    pub url: String,
}

#[derive(Clone, Serialize)]
pub struct TaskStatusPayload {
    pub task_id: String,
}

#[derive(Clone, Serialize)]
pub struct DownloadErrorPayload {
    pub task_id: String,
    pub message: String,
}


pub fn emit_progress(
    app: &AppHandle,
    task_id: &str,
    stage: &str,
    progress: f64,
    speed: String,
    eta: String,
    filename: String,
) {
    let _ = app.emit(
        "download-progress",
        ProgressPayload {
            task_id: task_id.to_string(),
            stage: stage.to_string(),
            progress,
            speed,
            eta,
            filename,
        },
    );
}

pub fn emit_done(app: &AppHandle, task_id: &str, output_path: &str, title: &str, url: &str) {
    let _ = app.emit(
        "download-done",
        DownloadDonePayload {
            task_id: task_id.to_string(),
            output_path: output_path.to_string(),
            title: title.to_string(),
            url: url.to_string(),
        },
    );
}

pub fn emit_cancelled(app: &AppHandle, task_id: &str) {
    let _ = app.emit(
        "download-cancelled",
        TaskStatusPayload {
            task_id: task_id.to_string(),
        },
    );
}

pub fn emit_paused(app: &AppHandle, task_id: &str) {
    let _ = app.emit(
        "download-paused",
        TaskStatusPayload {
            task_id: task_id.to_string(),
        },
    );
}

pub fn emit_error(app: &AppHandle, task_id: &str, message: String) {
    let _ = app.emit(
        "download-error",
        DownloadErrorPayload {
            task_id: task_id.to_string(),
            message,
        },
    );
}


pub fn strip_ansi(s: &str) -> String {
    // 简单移除 ANSI SGR 转义序列: ESC [ ... m
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            // 跳过 SGR 参数，直到终止字母（形如 ESC [ 31 m 的 'm'）
            for ch in chars.by_ref() {
                if ch.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// 解析 yt-dlp 进度行，stage 由调用方传入（不靠文件名猜）
/// 典型行：[download]   0.0% of   72.58MiB at    6.57MiB/s ETA 00:11
///         [download]  45.2% of 836.54MiB at  Unknown B/s ETA Unknown
pub fn parse_progress_line(line: &str, stage: &str) -> Option<ProgressPayload> {
    let line = strip_ansi(line).trim().to_string();
    if !line.contains("[download]") || !line.contains('%') {
        return None;
    }

    // 百分比取 '%' 前最后一个 token
    let pct_str = line.split('%').next()?;
    let pct = pct_str.split_whitespace().last()?;
    let progress: f64 = pct.parse().ok()?;

    // 速度：'at ' 之后第一个 token，Unknown 归一化为 -
    let speed = if let Some(idx) = line.find("at ") {
        let rest = &line[idx + 3..];
        let s = rest.split_whitespace().next().unwrap_or("-");
        if s.eq_ignore_ascii_case("unknown") {
            "-".to_string()
        } else {
            s.to_string()
        }
    } else {
        "-".to_string()
    };

    // ETA：'ETA ' 之后第一个 token，Unknown 归一化为 -
    let eta = if let Some(idx) = line.find("ETA ") {
        let rest = &line[idx + 4..];
        let s = rest.split_whitespace().next().unwrap_or("-");
        if s.eq_ignore_ascii_case("unknown") {
            "-".to_string()
        } else {
            s.to_string()
        }
    } else if line.contains("at ") {
        "00:00".to_string()
    } else {
        "-".to_string()
    };

    // 'of ' 之后是文件大小字符串（如 72.58MiB），非文件名；保留作 size 展示
    let size = if let Some(idx) = line.find("of ") {
        let rest = &line[idx + 3..];
        rest.split_whitespace().next().unwrap_or("-").to_string()
    } else {
        "-".to_string()
    };

    Some(ProgressPayload {
        task_id: String::new(),
        stage: stage.to_string(),
        progress,
        speed,
        eta,
        filename: size,
    })
}

/// 解析 ffmpeg stderr 进度行中的 time=HH:MM:SS.ms，返回秒数
pub fn parse_ffmpeg_time(line: &str) -> Option<f64> {
    let idx = line.find("time=")?;
    let rest = &line[idx + 5..];
    let time_str = rest.split_whitespace().next()?;
    let parts: Vec<&str> = time_str.split(':').collect();
    match parts.len() {
        3 => {
            let h: f64 = parts[0].parse().ok()?;
            let m: f64 = parts[1].parse().ok()?;
            let s: f64 = parts[2].parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + s)
        }
        2 => {
            let m: f64 = parts[0].parse().ok()?;
            let s: f64 = parts[1].parse().ok()?;
            Some(m * 60.0 + s)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_progress_line_basic() {
        let line = "[download]   45.2% of 836.54MiB at 10.5MiB/s ETA 00:23";
        let p = parse_progress_line(line, "video").expect("应解析成功");

        assert!((p.progress - 45.2).abs() < 0.01);
        assert_eq!(p.speed, "10.5MiB/s");
        assert_eq!(p.eta, "00:23");
        assert_eq!(p.filename, "836.54MiB");
        assert_eq!(p.stage, "video");
    }

    #[test]
    fn parse_progress_line_unknown_speed_eta() {
        // yt-dlp 早期进度行：speed/eta 尚未估算出
        let line = "[download]   0.0% of   72.58MiB at  Unknown B/s ETA Unknown";
        let p = parse_progress_line(line, "video").expect("应解析成功");
        assert!((p.progress - 0.0).abs() < 0.01);
        assert_eq!(p.speed, "-");
        assert_eq!(p.eta, "-");
    }

    #[test]
    fn parse_progress_line_audio_stage_passed_through() {
        let line = "[download]   78% of  45.00MiB at  5.0MiB/s ETA 00:11";
        let p = parse_progress_line(line, "audio").expect("应解析成功");
        assert_eq!(p.stage, "audio");
        assert!((p.progress - 78.0).abs() < 0.01);
    }

    #[test]
    fn parse_progress_line_no_download() {
        assert!(parse_progress_line("[info] Downloading... ", "video").is_none());
        assert!(parse_progress_line("  45% complete", "video").is_none());
        assert!(parse_progress_line("", "video").is_none());
    }

    #[test]
    fn parse_progress_line_eta_with_no_speed() {
        let line = "[download] 50% of 100.00MiB ETA 01:30";
        let p = parse_progress_line(line, "video").expect("应解析");
        assert!((p.progress - 50.0).abs() < 0.01);
        assert_eq!(p.speed, "-");
        assert_eq!(p.eta, "01:30");
    }

    #[test]
    fn parse_progress_line_no_eta_no_speed() {
        let line = "[download] 100% of 836.54MiB";
        let p = parse_progress_line(line, "video").expect("应解析");
        assert_eq!(p.progress, 100.0);
        assert_eq!(p.speed, "-");
        assert_eq!(p.eta, "-");
    }
}


