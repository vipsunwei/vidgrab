//! 任务与历史持久化命令：tasks.json（未完成任务）/ history.json（完成记录）。
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::naming::{compute_titles, with_quality_tag};
use crate::state::TaskOutputs;

/// 只保存暂停/下载中的任务，用于应用重启后恢复断点续传。
#[tauri::command]
pub async fn load_tasks(app: AppHandle) -> String {
    let dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return "[]".to_string(),
    };
    match std::fs::read_to_string(dir.join("tasks.json")) {
        Ok(s) if !s.trim().is_empty() => s,
        _ => "[]".to_string(),
    }
}

#[tauri::command]
pub async fn save_tasks(app: AppHandle, json: String) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("tasks.json"), json).map_err(|e| e.to_string())
}

/// 应用重启后恢复的暂停任务重新登记输出位置，
/// 这样「删除任务」仍能找到并清理对应的 .part 残留。
#[tauri::command]
pub fn register_task_output(
    task_id: String,
    output_dir: String,
    title: String,
    output_name: Option<String>,
    quality_tag: Option<String>,
    task_outputs: State<'_, TaskOutputs>,
) {
    let safe_title = compute_titles(&title, output_name.as_deref()).1;
    let safe_title = with_quality_tag(&safe_title, quality_tag.as_deref());
    task_outputs
        .0
        .lock()
        .unwrap()
        .insert(task_id, (PathBuf::from(output_dir), safe_title));
}


/// 读取下载历史（存于 app data 目录的 history.json）。文件不存在或损坏时返回空数组 JSON。
#[tauri::command]
pub async fn load_history(app: AppHandle) -> String {
    let dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return "[]".to_string(),
    };
    let path = dir.join("history.json");
    match std::fs::read_to_string(&path) {
        Ok(s) if !s.trim().is_empty() => s,
        _ => "[]".to_string(),
    }
}

/// 写入下载历史到 app data 目录的 history.json（覆盖式）。
#[tauri::command]
pub async fn save_history(app: AppHandle, json: String) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("history.json");
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// 文件不存在视为已删除，返回 Ok；其他错误返回错误信息。
#[tauri::command]
pub async fn delete_file(path: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("路径为空".to_string());
    }
    match std::fs::remove_file(&path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("删除文件失败: {}", e)),
    }
}

/// 重新下载前的清理：删除成品文件及其同目录同前缀的 .part / .part-Frag 碎片。
/// 位于 history 域（与 delete_file 同族），便于复用文件操作与命令注册。
/// 传入的是成品路径（来自历史记录的 outputPath）；前缀取文件名 stem，
/// 覆盖单轨（{prefix}.mp4.part）与双轨（{prefix}.v.*.part）形态。
#[tauri::command]
pub async fn redownload_cleanup(output_path: String) -> Result<(), String> {
    if output_path.is_empty() {
        return Ok(());
    }
    let path = PathBuf::from(&output_path);
    // 删除成品（不存在也视为成功）
    if path.exists() {
        if let Err(e) = std::fs::remove_file(&path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(format!("删除旧文件失败: {}", e));
            }
        }
    }
    // 清理同名碎片：前缀 = 成品文件名去扩展名
    let Some(dir) = path.parent() else {
        return Ok(());
    };
    let Some(stem) = path.file_stem() else {
        return Ok(());
    };
    let prefix = format!("{}.", stem.to_string_lossy());
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.filter_map(|e| e.ok()) {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix)
                && (name.ends_with(".part") || name.contains(".part-Frag"))
            {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    Ok(())
}
