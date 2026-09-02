//! 下载任务编排：启动/暂停/取消/删除命令，以及一次运行的整体流程（解析 → 双轨 → 合并）。

use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::downloader::{find_ytdlp, run_ytdlp_dump};
use crate::meta::parse_ytdlp_output;
use crate::naming::find_latest_in_dir;
use crate::proc::stop_task;
use crate::progress::{emit_cancelled, emit_done, emit_error, emit_paused};
use crate::state::{
    is_cancelled, is_paused, next_run_seq, CancelSet, PausingSet, TaskOutputs, TaskSeqs, TaskTable,
};
use crate::track::{merge_with_ffmpeg, run_track_with_retry};
use crate::url::clean_url;

/// 启动下载。立即返回，实际流程在后台任务中跑，状态一律通过事件回传。
// 参数个数由「前端字段 + 各 State 注入」决定，无法再拆分，故豁免
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn start_download(
    url: String,
    format_ids: Vec<String>,
    output_dir: String,
    output_name: Option<String>,
    quality_tag: Option<String>,
    cookie_source: Option<String>,
    task_id: String,
    app: AppHandle,
    tasks: State<'_, TaskTable>,
    cancelling: State<'_, CancelSet>,
    pausing: State<'_, PausingSet>,
    task_seqs: State<'_, TaskSeqs>,
    task_outputs: State<'_, TaskOutputs>,
) -> Result<(), String> {
    let app_c = app.clone();
    let tasks_c = tasks.inner().clone();
    let cancelling_c = CancelSet(cancelling.inner().0.clone());
    let pausing_c = PausingSet(pausing.inner().0.clone());
    let seqs_c = TaskSeqs(task_seqs.inner().0.clone());
    let outputs_c = TaskOutputs(task_outputs.inner().0.clone());
    // 分配运行代号：该次运行的迟到事件会被 seq 守卫静默丢弃
    let seq = next_run_seq();
    seqs_c.0.lock().unwrap().insert(task_id.clone(), seq);
    tokio::spawn(async move {
        run_download_task(
            task_id, url, format_ids, output_dir, output_name, quality_tag, cookie_source,
            app_c, tasks_c, cancelling_c, pausing_c, seqs_c, outputs_c, seq,
        )
        .await;
    });
    Ok(())
}

/// 一次运行的完整流程：在后台任务中执行，结束/出错时通过事件通知前端。
#[allow(clippy::too_many_arguments)]
pub async fn run_download_task(
    task_id: String,
    url: String,
    format_ids: Vec<String>,
    output_dir: String,
    output_name: Option<String>,
    quality_tag: Option<String>,
    cookie_source: Option<String>,
    app: AppHandle,
    tasks: TaskTable,
    cancelling: CancelSet,
    pausing: PausingSet,
    task_seqs: TaskSeqs,
    task_outputs: TaskOutputs,
    seq: u64,
) {
    let result: Result<(String, String), String> = (async {
        let ytdlp = find_ytdlp()
            .ok_or_else(|| "未找到 yt-dlp（打包版本异常）".to_string())?;

        let url = clean_url(&url);
        let cookie = cookie_source.as_deref();
        let json_str = run_ytdlp_dump(&url, cookie)?;
        let video_info = parse_ytdlp_output(&json_str, &url)?;
        // 文件名基础：用户自定义名优先（去扩展名），否则用视频标题；统一 sanitize + 截断
        let (base_title, safe_title) =
            crate::naming::compute_titles(&video_info.title, output_name.as_deref());
        // 追加质量标签：同视频不同清晰度/音质落在不同文件名，可并行下载互不覆盖
        let safe_title = crate::naming::with_quality_tag(&safe_title, quality_tag.as_deref());

        let output_dir = if output_dir.is_empty() {
            dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("VidGrab")
        } else {
            PathBuf::from(&output_dir)
        };
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("创建目录失败: {}", e))?;

        let output_base = output_dir.join(&safe_title);
        // 记录输出位置，供「删除任务」清理 .part 残留
        task_outputs
            .0
            .lock()
            .unwrap()
            .insert(task_id.clone(), (output_dir.clone(), safe_title.clone()));

        if format_ids.len() >= 2 {
            // 双轨下载：视频 + 音频
            let vid_fmt = &format_ids[0];
            let aud_fmt = &format_ids[1];
            let vid_out = format!("{}.v.%(ext)s", output_base.display());
            let aud_out = format!("{}.a.%(ext)s", output_base.display());

            run_track_with_retry(
                &ytdlp, &url, vid_fmt, "video", &app, &vid_out, &task_id, &tasks, &pausing,
                &cancelling, cookie, seq,
            )
            .await?;
            // 本轨已结束：撤下 PID 登记，避免双轨空窗期拿着已退出进程的 PID 去树杀
            // （该 PID 可能已被系统分配给别的进程）
            tasks.lock().await.remove(&task_id);
            // 视频轨结束到音频轨启动之间存在空窗，此处不检查暂停/取消的话，
            // 会出现「点了暂停却继续下载到完成」
            if is_paused(&pausing, &task_id, seq).await {
                return Err("__paused__".to_string());
            }
            if is_cancelled(&cancelling, &task_id, seq).await {
                return Err("__cancelled__".to_string());
            }
            run_track_with_retry(
                &ytdlp, &url, aud_fmt, "audio", &app, &aud_out, &task_id, &tasks, &pausing,
                &cancelling, cookie, seq,
            )
            .await?;
            tasks.lock().await.remove(&task_id);

            let merged = format!("{}.mp4", output_base.display());
            let vid_path = find_latest_in_dir(&output_dir, &safe_title, ".v.")?;
            let aud_path = find_latest_in_dir(&output_dir, &safe_title, ".a.")?;

            if let Err(e) = merge_with_ffmpeg(
                &vid_path,
                &aud_path,
                &merged,
                video_info.duration,
                &app,
                &task_id,
                &tasks,
            )
            .await
            {
                // 合并没走完，{base}.mp4 是个残缺文件。留着它，下次同一标题的任务
                // 会被 find_latest_in_dir 按 mtime 优先捡成「成品」，必须删
                let _ = std::fs::remove_file(&merged);
                return Err(e);
            }

            let _ = std::fs::remove_file(&vid_path);
            let _ = std::fs::remove_file(&aud_path);

            // 文件名用截断后的 safe_title，展示标题用用户自定义名（无则为视频标题）
            Ok((merged, base_title.clone()))
        } else {
            // 单轨下载
            let fmt = format_ids.first().ok_or("未指定格式")?;
            let out = format!("{}.%(ext)s", output_base.display());
            if is_paused(&pausing, &task_id, seq).await {
                return Err("__paused__".to_string());
            }
            if is_cancelled(&cancelling, &task_id, seq).await {
                return Err("__cancelled__".to_string());
            }
            run_track_with_retry(
                &ytdlp, &url, fmt, "video", &app, &out, &task_id, &tasks, &pausing, &cancelling,
                cookie, seq,
            )
            .await?;
            tasks.lock().await.remove(&task_id);

            let final_path = find_latest_in_dir(&output_dir, &safe_title, "")?;
            // 文件名用截断后的 safe_title，展示标题用用户自定义名（无则为视频标题）
            Ok((final_path, base_title.clone()))
        }
    })
    .await;

    let success = result.is_ok();
    // 只让最后一次启动的运行发事件、做清理：快速「取消→重新下载」时，
    // 旧运行的迟到事件（cancelled/paused/done…）不得覆盖新运行的状态
    let is_current = task_seqs.0.lock().unwrap().get(&task_id) == Some(&seq);
    // 取消/暂停标记按运行代号隔离读取：只响应属于自己那次启动的标记
    let cancelled = is_cancelled(&cancelling, &task_id, seq).await;
    let paused = is_paused(&pausing, &task_id, seq).await;
    match result {
        Ok((path, title)) => {
            if is_current {
                emit_done(&app, &task_id, &path, &title, &url);
            }
        }
        Err(e) => {
            if !is_current {
                // 旧运行静默收场，不打扰新运行
            } else if paused {
                emit_paused(&app, &task_id);
            } else if cancelled {
                emit_cancelled(&app, &task_id);
            } else {
                emit_error(&app, &task_id, e);
            }
        }
    }
    if is_current {
        tasks.lock().await.remove(&task_id);
        if cancelled {
            cancelling.0.lock().await.remove(&task_id);
        }
        if paused {
            pausing.0.lock().await.remove(&task_id);
        }
        // 成功后输出位置已无用（.part 已被消费/删除），仅失败、暂停、取消时保留供删除清理
        if success {
            task_outputs.0.lock().unwrap().remove(&task_id);
        } else if !paused {
            // 取消/失败：清掉双轨中间文件。暂停要留着续传，见 clean_track_files 注释
            let out = task_outputs.0.lock().unwrap().get(&task_id).cloned();
            if let Some((dir, prefix)) = out {
                clean_all_fragments(&dir, &prefix);
            }
        }
    } else {
        // 旧运行只清走与自身代号匹配的标记，不碰新运行的状态
        if cancelled {
            cancelling.0.lock().await.remove(&task_id);
        }
        if paused {
            pausing.0.lock().await.remove(&task_id);
        }
    }
}

/// 取消下载：终止进程并丢弃 .part，收尾时据此发 cancelled 事件而非 error。
#[tauri::command]
pub async fn cancel_download(
    task_id: String,
    tasks: State<'_, TaskTable>,
    cancelling: State<'_, CancelSet>,
    task_seqs: State<'_, TaskSeqs>,
) -> Result<(), String> {
    // 标记绑定最后一次启动的运行代号：旧运行的收尾不会误清新运行的标记
    let seq = task_seqs.inner().0.lock().unwrap().get(&task_id).copied();
    if let Some(seq) = seq {
        cancelling.inner().0.lock().await.insert(task_id.clone(), seq);
    }
    stop_task(&tasks, &task_id).await;
    Ok(())
}

/// 暂停下载：终止子进程但保留 .part 断点。
/// 「继续下载」= 前端用原参数重新调用 start_download，yt-dlp 检测到同名
/// .part 自动断点续传，无需后端额外状态。
#[tauri::command]
pub async fn pause_download(
    task_id: String,
    tasks: State<'_, TaskTable>,
    pausing: State<'_, PausingSet>,
    task_seqs: State<'_, TaskSeqs>,
) -> Result<(), String> {
    let seq = task_seqs.inner().0.lock().unwrap().get(&task_id).copied();
    if let Some(seq) = seq {
        pausing.inner().0.lock().await.insert(task_id.clone(), seq);
    }
    stop_task(&tasks, &task_id).await;
    Ok(())
}

/// 清理某任务输出目录中的临时碎片：以 {prefix}. 开头，且属于以下任一：
/// - 双轨中间文件（{prefix}.v.* / {prefix}.a.*）
/// - 单轨/双轨的 .part 断点（含 yt-dlp 分片 .part-FragN）
/// 不碰成品（{prefix}.mp4 等），也不会误删其他任务（前缀含任务专属的 quality tag）。
/// 取消/失败与显式删除（delete_task / clear_task_part）统一调用，避免单轨 .part 漏清。
/// 暂停时不调本函数——那是断点续传的原料，要留着。
fn clean_all_fragments(dir: &std::path::Path, prefix: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let base_prefix = format!("{prefix}.");
    for e in entries.filter_map(|e| e.ok()) {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.starts_with(&base_prefix) {
            continue;
        }
        let after = &name[base_prefix.len()..];
        // 双轨中间文件
        let is_track = after.starts_with("v.") || after.starts_with("a.");
        // 单轨 .part / 双轨分片 .part-FragN
        let is_part = name.ends_with(".part") || name.contains(".part-Frag");
        if is_track || is_part {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// 删除任务：若仍在下载则终止进程，并清理该任务已下载的 .part 残留文件。
/// （.part 按输出目录 + 文件名前缀匹配，不会误删其他任务或历史成品）
#[tauri::command]
pub async fn delete_task(
    task_id: String,
    tasks: State<'_, TaskTable>,
    cancelling: State<'_, CancelSet>,
    task_seqs: State<'_, TaskSeqs>,
    task_outputs: State<'_, TaskOutputs>,
) -> Result<(), String> {
    let running = tasks.inner().lock().await.get(&task_id).is_some();
    if running {
        let seq = task_seqs.inner().0.lock().unwrap().get(&task_id).copied();
        if let Some(seq) = seq {
            cancelling.inner().0.lock().await.insert(task_id.clone(), seq);
        }
        stop_task(&tasks, &task_id).await;
    }
    let out = task_outputs.inner().0.lock().unwrap().remove(&task_id);
    if out.is_some() {
        // 进程终止后文件句柄释放需要一点时间，稍等再删。挪到后台跑，
        // 不然命令要挂满这半秒才回包，删除按钮按下去是粘的
        tokio::spawn(async move {
            if running {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            if let Some((dir, prefix)) = out {
                clean_all_fragments(&dir, &prefix);
            }
        });
    }
    Ok(())
}

/// 仅清理某任务的 .part 碎片，不终止进程、不删任务。
/// 「重新下载」前调用：让重下从头开始而非续传那段可能已损坏的半成品。
#[tauri::command]
pub async fn clear_task_part(
    task_id: String,
    task_outputs: State<'_, TaskOutputs>,
) -> Result<(), String> {
    let out = task_outputs.inner().0.lock().unwrap().get(&task_id).cloned();
    if let Some((dir, prefix)) = out {
        clean_all_fragments(&dir, &prefix);
    }
    Ok(())
}
