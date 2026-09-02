//! 单轨执行与合并：网络错误分类、自动重试（断点续传）、ffmpeg 合并。
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use tauri::AppHandle;
use tokio::io::BufReader;
use tokio::process::Command;

use crate::downloader::{find_ffmpeg, push_cookie_args};
use crate::proc::{force_utf8_env, hide_window_tokio, kill_pid_tree, read_line_lossy};
#[cfg(windows)]
use crate::proc::attach_kill_on_close_job;
use crate::progress::{emit_progress, parse_ffmpeg_time, parse_progress_line};
use crate::state::{is_cancelled, is_paused, CancelSet, PausingSet, TaskTable};

pub fn is_network_error(msg: &str) -> bool {
    let m = msg.to_lowercase();
    if [
        "http error 404",
        "http error 410",
        "http error 451",
        "not a valid url",
        "unsupported url",
        "private video",
        "video unavailable",
        "requested format is not available",
    ]
    .iter()
    .any(|k| m.contains(k))
    {
        return false;
    }
    [
        "connection reset",
        "connection aborted",
        "connection refused",
        "timed out",
        "timeout",
        "temporary failure",
        "getaddrinfo",
        "name or service not known",
        "unable to download webpage",
        "network is down",
        "network is unreachable",
        "read error",
        "incomplete read",
        "http error 500",
        "http error 502",
        "http error 503",
        "precondition failed",
    ]
    .iter()
    .any(|k| m.contains(k))
        || has_winsock_code(&m)
}

// Windows socket 错误码（10054 连接被重置 / 10060 连接超时）。
// 裸匹配数字会误伤：文件大小、时长里恰好出现同串数字也会被当成网络错误，白等满 6 次退避。
// 所以要求它们以 errno / winerror / Python 异常元组的形式出现才算数
fn has_winsock_code(m: &str) -> bool {
    [
        "errno 10054",
        "winerror 10054",
        "(10054,",
        "errno 10060",
        "winerror 10060",
        "(10060,",
    ]
    .iter()
    .any(|k| m.contains(k))
}

/// 判定是否「断点失效」类错误（HTTP 416：.part 大小与服务端错位）。
/// 此时续传必然再次 416，必须删除 .part 从头下载。
/// 注意：不能裸匹配 "416"——文件大小/时长里出现同串数字会误判为断点失效，
/// 白删 .part 从头下载。对齐 is_network_error 里 has_winsock_code 的教训：用精确上下文。
pub fn is_unsatisfiable_range(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("http error 416") || m.contains("requested range not satisfiable")
}

/// 删除某输出模板对应的 .part 断点文件（含分片 .part-FragN）。
/// output_template 形如 {目录}\{前缀}.v.%(ext)s
pub async fn delete_part_files(output_template: &str) {
    let Some(stripped) = output_template.strip_suffix("%(ext)s") else {
        return;
    };
    let Some(sep) = stripped.rfind(['\\', '/']) else {
        return;
    };
    let (dir, prefix) = (&stripped[..sep], &stripped[sep + 1..]);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(|e| e.ok()) {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with(prefix)
            && (name.ends_with(".part") || name.contains(".part-Frag"))
        {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// 带断网自动重试的单轨下载：网络类失败按退避自动续传重试
/// （每次重启 yt-dlp 都从 .part 断点接着下），重试等待期间暂停/取消随时可打断。
#[allow(clippy::too_many_arguments)]
pub async fn run_track_with_retry(
    ytdlp: &PathBuf,
    url: &str,
    format_id: &str,
    stage: &str,
    app: &AppHandle,
    output_template: &str,
    task_id: &str,
    tasks: &TaskTable,
    pausing: &PausingSet,
    cancelling: &CancelSet,
    cookie: Option<&str>,
    seq: u64,
) -> Result<(), String> {
    const MAX_RETRIES: u32 = 6;
    let mut attempt: u32 = 0;
    loop {
        let result = run_ytdlp_with_progress(
            ytdlp,
            url,
            format_id,
            stage,
            app,
            output_template,
            task_id,
            tasks,
            pausing,
            cancelling,
            cookie,
            seq,
        )
        .await;
        match result {
            Ok(()) => return Ok(()),
            Err(e) => {
                let unsatisfiable = is_unsatisfiable_range(&e);
                attempt += 1;
                if attempt > MAX_RETRIES {
                    return Err(e);
                }
                if unsatisfiable {
                    // 断点已毒化：删除 .part 后从头下载（自动自愈，无需用户手动清理）
                    delete_part_files(output_template).await;
                    emit_progress(
                        app,
                        task_id,
                        stage,
                        0.0,
                        "断点失效，重新下载".into(),
                        "-".into(),
                        "-".into(),
                    );
                } else if !is_network_error(&e) {
                    return Err(e);
                }
                // 退避：5s/20s/45s/60s/60s/60s，总窗口约 4 分钟
                let wait_secs = std::cmp::min(5 * attempt * attempt, 60);
                // 进度值传 0 由前端忽略（避免把进度条打回 0），重试提示走速度位
                emit_progress(
                    app,
                    task_id,
                    stage,
                    0.0,
                    format!("网络重试 {attempt}/{MAX_RETRIES}"),
                    "-".into(),
                    "-".into(),
                );
                for _ in 0..wait_secs * 2 {
                    if is_paused(pausing, task_id, seq).await {
                        return Err("__paused__".to_string());
                    }
                    if is_cancelled(cancelling, task_id, seq).await {
                        return Err("__cancelled__".to_string());
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
}



/// 单轨下载：启动 yt-dlp 子进程，逐行解析进度回传，按退出码判定成败。
/// seq 为本次运行代号，暂停/取消标记按它严格匹配（见 state.rs 注释）。
#[allow(clippy::too_many_arguments)]
pub async fn run_ytdlp_with_progress(
    ytdlp: &PathBuf,
    url: &str,
    format_id: &str,
    stage: &str,
    app: &AppHandle,
    output_template: &str,
    task_id: &str,
    tasks: &TaskTable,
    pausing: &PausingSet,
    cancelling: &CancelSet,
    cookie_source: Option<&str>,
    seq: u64,
) -> Result<(), String> {
    // spawn 前最后一次检查：消除检查点之后、进程启动之前点击暂停/取消的竞态窗口
    if is_paused(pausing, task_id, seq).await {
        return Err("__paused__".to_string());
    }
    if is_cancelled(cancelling, task_id, seq).await {
        return Err("__cancelled__".to_string());
    }

    let mut args: Vec<String> = vec![
        "-f".into(),
        format_id.to_string(),
        "--progress".into(),
        "--newline".into(),
        "--no-colors".into(),
        // 与解析路径（run_ytdlp_dump）保持一致：多 P 视频只下载用户所选/解析的
        // 那一个视频，而不是把整个分 P 播放列表全部下载
        "--no-playlist".into(),
        "-o".into(),
        output_template.to_string(),
    ];
    if let Some(src) = cookie_source {
        if !src.is_empty() {
            // 与解析路径共用 push_cookie_args
            push_cookie_args(&mut args, src);
        }
    }
    args.push("--".into());
    args.push(url.to_string());

    // unix：设置独立进程组，停止时整组击杀（PyInstaller 为父子进程结构）
    #[allow(unused_mut)]
    let mut std_spawn = std::process::Command::new(ytdlp);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        std_spawn.process_group(0);
    }
    let mut spawn = Command::from(std_spawn);
    spawn.args(&args);
    // yt-dlp 是 Python 程序：Windows 下 stdout 为管道时默认用本地编码（GBK），
    // 中文进度行会变成非 UTF-8 字节，配合 lossy 读取双保险
    force_utf8_env(&mut spawn);
    hide_window_tokio(&mut spawn);
    let mut child = spawn
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 yt-dlp 失败: {}", e))?;

    // yt-dlp 新版进度行输出到 stdout（非 stderr）
    let pid = child.id();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // 登记子进程 PID（停止时树杀用）；子进程对象由本任务独占，不共享互斥锁
    tasks.lock().await.insert(task_id.to_string(), pid);
    // 补第二道检查：spawn 到 PID 登记之间有个窗口，用户恰好在这瞬间点的暂停/取消，
    // stop_task 查表时还没有这个 PID，就杀不掉，yt-dlp 会一路跑到下载完成
    if let Some(p) = pid {
        let paused = is_paused(pausing, task_id, seq).await;
        let cancelled = is_cancelled(cancelling, task_id, seq).await;
        if paused || cancelled {
            kill_pid_tree(p).await;
            let _ = child.wait().await;
            return Err(if paused {
                "__paused__".to_string()
            } else {
                "__cancelled__".to_string()
            });
        }
    }
    // 应用退出时自动终止下载进程树（见 attach_kill_on_close_job 注释）
    #[cfg(windows)]
    attach_kill_on_close_job(&child);

    let app_clone = app.clone();
    let stage_c = stage.to_string();
    let task_id_c = task_id.to_string();
    // 读 stdout 进度行，解析后发事件
    let progress_handle = tokio::spawn(async move {
        let mut r = stdout_reader;
        let mut line_buf = Vec::new();
        while let Ok(Some(line)) = read_line_lossy(&mut r, &mut line_buf).await {
            if let Some(payload) = parse_progress_line(&line, &stage_c) {
                if payload.progress > 0.0 || payload.speed != "-" {
                    emit_progress(
                        &app_clone,
                        &task_id_c,
                        &payload.stage,
                        payload.progress,
                        payload.speed,
                        payload.eta,
                        payload.filename,
                    );
                }
            }
        }
    });

    // 收集 stderr 用于失败时报错
    let err_handle = tokio::spawn(async move {
        let mut r = stderr_reader;
        let mut buf: Vec<String> = Vec::new();
        let mut line_buf = Vec::new();
        while let Ok(Some(line)) = read_line_lossy(&mut r, &mut line_buf).await {
            if !line.trim().is_empty() {
                buf.push(line);
                if buf.len() > 12 {
                    buf.remove(0);
                }
            }
        }
        buf
    });

    // 等进程结束（子进程对象独占，无锁竞争；stop_task 树杀后此 wait 立即返回）
    let status = child
        .wait()
        .await
        .map_err(|e| format!("yt-dlp 进程出错: {}", e))?;

    // 等 reader 任务结束
    let _ = progress_handle.await;
    let last_stderr = err_handle.await.unwrap_or_default();

    if !status.success() {
        let tail = last_stderr
            .iter()
            .filter(|l| {
                let l = l.to_lowercase();
                l.contains("error") || l.contains("sign in") || l.contains("cookie")
            })
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let hint = if tail.is_empty() {
            "请检查网络或视频是否可用".to_string()
        } else {
            tail
        };
        return Err(format!("yt-dlp 失败（退出码 {}）：\n{}", status, hint));
    }

    Ok(())
}

/// 判断音频编码是否可直接 copy 进 mp4 容器（零重编码）。
/// mp4a 系列（如 mp4a.40.2）与裸 "aac" 都算；opus/mp3 等需兜底重编码为 aac。
pub fn is_mp4_compatible_audio(codec: &str) -> bool {
    let c = codec.to_lowercase();
    c.contains("mp4a") || c == "aac"
}

pub async fn merge_with_ffmpeg(
    video_path: &str,
    audio_path: &str,
    output_path: &str,
    duration_secs: u32,
    app: &AppHandle,
    task_id: &str,
    tasks: &TaskTable,
    audio_codec: &str,
) -> Result<(), String> {
    let ffmpeg =
        find_ffmpeg().ok_or_else(|| "未找到 ffmpeg（打包版本异常）".to_string())?;

    emit_progress(app, task_id, "merge", 0.0, "-".into(), "-".into(), "Merging...".into());

    #[allow(unused_mut)]
    let mut std_spawn = std::process::Command::new(&ffmpeg);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        std_spawn.process_group(0);
    }
    let mut spawn = Command::from(std_spawn);
    // 音频：mp4 兼容源（aac/m4a）直接 copy 零重编码；否则兜底重编码为 aac
    let audio_mp4_compatible =
        is_mp4_compatible_audio(audio_codec) || audio_path.to_lowercase().ends_with(".m4a");
    let audio_args: &[&str] = if audio_mp4_compatible {
        &["-c:a", "copy"]
    } else {
        &["-c:a", "aac", "-strict", "experimental"]
    };
    let mut args: Vec<&str> = vec!["-y", "-i", video_path, "-i", audio_path, "-c:v", "copy"];
    args.extend_from_slice(audio_args);
    // 若视频轨因断点续传损坏而略短于音频轨，-shortest 让输出在视频结束时
    // 收尾，避免出现画面冻结在最后一帧、声音继续播的坏文件
    args.extend_from_slice(&["-shortest", "-progress", "pipe:1", output_path]);
    spawn.args(args);
    force_utf8_env(&mut spawn);
    hide_window_tokio(&mut spawn);
    let mut child = spawn
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 ffmpeg 失败: {}", e))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut err_reader = BufReader::new(stderr);

    // 登记子进程 PID（停止时树杀用）；子进程对象由本任务独占，不共享互斥锁
    let pid = child.id();
    tasks.lock().await.insert(task_id.to_string(), pid);
    // 应用退出时自动终止合并进程（见 attach_kill_on_close_job 注释）
    #[cfg(windows)]
    attach_kill_on_close_job(&child);

    let app_clone = app.clone();
    let duration = duration_secs.max(1) as f64;
    let task_id_c = task_id.to_string();

    // 读取进度输出
    let progress_handle = tokio::spawn(async move {
        let mut last_progress = 0.0;
        let mut line_buf = Vec::new();
        while let Ok(Some(line)) = read_line_lossy(&mut reader, &mut line_buf).await {
            if let Some(t) = parse_ffmpeg_time(&line) {
                let pct = ((t / duration) * 100.0).min(99.9).max(last_progress);
                last_progress = pct;
                emit_progress(
                    &app_clone,
                    &task_id_c,
                    "merge",
                    pct,
                    "-".into(),
                    "-".into(),
                    "Merging...".into(),
                );
            }
        }
    });

    // 收集 stderr 用于报错
    let err_handle = tokio::spawn(async move {
        let mut buf = String::new();
        let mut line_buf = Vec::new();
        while let Ok(Some(line)) = read_line_lossy(&mut err_reader, &mut line_buf).await {
            buf.push_str(&line);
            buf.push('\n');
        }
        buf
    });

    // 合并超时兜底：ffmpeg 偶发挂死（损坏输入/编码器死锁）会让 child.wait()
    // 永久挂起，拖垮整个下载任务且 PID 常驻 tasks。按视频时长给宽容上限并封顶。
    let merge_secs = ((duration_secs as u64) * 5 + 120).min(1800);
    let merge_timeout = std::time::Duration::from_secs(merge_secs);
    let status = match tokio::time::timeout(merge_timeout, child.wait()).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let _ = progress_handle.abort();
            let _ = err_handle.abort();
            return Err(format!("ffmpeg 进程出错: {}", e));
        }
        Err(_) => {
            // 杀掉 ffmpeg 进程组（含其可能 fork 的子进程），避免孤儿残留
            if let Some(p) = pid {
                kill_pid_tree(p).await;
            }
            let _ = progress_handle.abort();
            let _ = err_handle.abort();
            return Err(format!("ffmpeg 合并超时（>{}s），已终止进程", merge_secs));
        }
    };
    let (_, stderr_log) = tokio::join!(progress_handle, err_handle);
    let stderr_log = stderr_log.unwrap_or_default();
    if !status.success() {
        return Err(format!("ffmpeg 合并失败: {}", stderr_log));
    }

    emit_progress(app, task_id, "merge", 100.0, "-".into(), "-".into(), "Merging...".into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_error_matches_real_network_failures() {
        assert!(is_network_error("Connection reset by peer"));
        assert!(is_network_error("[Errno 10054] 远程主机强迫关闭"));
        assert!(is_network_error("ConnectionResetError(10054, 'x', None, 10054, None)"));
        assert!(is_network_error("[WinError 10060] A connection attempt failed"));
    }

    #[test]
    fn network_error_ignores_bare_numbers() {
        // 文件大小、时长里恰好带这几位数字，不该被当成网络错误白等满 6 次退避
        assert!(!is_network_error("downloading 10054 bytes"));
        assert!(!is_network_error("duration 10060 seconds"));
    }

    #[test]
    fn network_error_excludes_permanent_failures() {
        // 这几类重试也没用，必须判为非网络错误直接失败
        assert!(!is_network_error("HTTP Error 404: Not Found"));
        assert!(!is_network_error("Private video"));
        assert!(!is_network_error("Video unavailable"));
    }

    #[test]
    fn unsatisfiable_range_detected() {
        assert!(is_unsatisfiable_range("HTTP Error 416: Requested Range Not Satisfiable"));
        assert!(!is_unsatisfiable_range("HTTP Error 404"));
    }

    #[test]
    fn unsatisfiable_range_ignores_bare_numbers() {
        // 文件大小、时长里恰好带 416 这几位数字，不该被当成断点失效白删 .part
        assert!(!is_unsatisfiable_range("downloading 4160 bytes"));
        assert!(!is_unsatisfiable_range("duration 416 seconds"));
    }

    #[test]
    fn mp4_compatible_audio_covers_mp4a_and_bare_aac() {
        // 多数站点标 mp4a.40.2；少数站点 acodec 恰为裸 "aac"，两种都应零重编码 copy
        assert!(is_mp4_compatible_audio("mp4a.40.2"));
        assert!(is_mp4_compatible_audio("MP4A.40.2"));
        assert!(is_mp4_compatible_audio("aac"));
        assert!(is_mp4_compatible_audio("AAC"));
    }

    #[test]
    fn mp4_compatible_audio_excludes_opus_and_mp3() {
        // opus/mp3 走兜底 aac 重编码：保守取舍，保证 mp4 容器兼容性，非 bug。
        // 此处的预期同时锁定了 mp3 当前走重编码的行为，避免未来被误改成 copy。
        assert!(!is_mp4_compatible_audio("opus"));
        assert!(!is_mp4_compatible_audio("mp3"));
        assert!(!is_mp4_compatible_audio(""));
    }
}

