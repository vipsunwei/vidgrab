//! VidGrab 应用入口与装配层。
//! 业务实现按域拆分：downloader（yt-dlp 封装）/ download（下载引擎与命令）/
//! install（运行时安装器）/ history（持久化命令）/ proc（进程控制原语）/ state（共享状态）。

mod cookies;
mod downloader;
mod download;
mod history;
mod install;
mod meta;
mod naming;
mod proc;
mod progress;
mod state;
mod track;
mod url;

use std::sync::Mutex;
use tauri::{Manager, WindowEvent};

use state::LastMonitor;

/// 返回系统默认下载目录（Downloads/VidGrab）
#[tauri::command]
fn get_default_download_dir() -> Result<String, String> {
    let path = dirs::download_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("VidGrab");
    Ok(path.to_string_lossy().to_string())
}

/// 返回真实版本号（来自 Cargo.toml，与 tauri.conf.json 的 version 同步）
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(state::TaskTable::default())
        .manage(state::CancelSet::default())
        .manage(state::PausingSet::default())
        .manage(state::TaskSeqs::default())
        .manage(state::TaskOutputs::default())
        .manage(LastMonitor(Mutex::new(String::new())))
        .on_window_event(|window, event| {
            // 跨显示器拖动时只做「放不下才收缩 + 居中」，用户调整过的尺寸尽量保留
            // （平时的大小/位置由 window-state 插件记忆）。单屏内拖动不触发，避免抖动。
            if let WindowEvent::Moved(_) = event {
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let id = format!("{}x{}", monitor.position().x, monitor.position().y);
                    let changed = {
                        let state = window.state::<LastMonitor>();
                        let mut last = state.0.lock().unwrap();
                        let changed = *last != id;
                        *last = id.clone();
                        changed
                    };
                    if changed {
                        if let Ok(size) = window.outer_size() {
                            let msize = monitor.size();
                            // 工作区留出任务栏余量；放不下才收缩，物理像素口径一致
                            let w = size.width.min(msize.width.saturating_sub(32)).max(600);
                            let h = size.height.min(msize.height.saturating_sub(80)).max(420);
                            if w != size.width || h != size.height {
                                let _ = window.set_size(tauri::PhysicalSize::new(w, h));
                            }
                        }
                        let _ = window.center();
                    }
                }
            }
        })
        .setup(|app| {
            // 注入跨平台二进制查找目录：
            // - macOS 资源在 Contents/Resources，可执行文件在 Contents/MacOS，不同级
            // - Linux deb 的资源在 /usr/lib 下
            // - bin/ 作为 bundle resources 打包后，可能落在 resource_dir 根，
            //   也可能保留 bin/ 子目录结构，两处都查以免漏
            // - 运行时安装目录（app_data/bin）也要能被找到
            let mut dirs: Vec<std::path::PathBuf> = Vec::new();
            if let Ok(d) = app.path().resource_dir() {
                dirs.push(d.join("bin"));
                dirs.push(d);
            }
            if let Ok(d) = app.path().app_data_dir() {
                dirs.push(d.join("bin"));
            }
            downloader::set_search_dirs(dirs);

            // 窗口大小/位置由 window-state 插件记忆并恢复（首次启动用
            // tauri.conf.json 的默认尺寸），此处不再强制设置

            if let Ok(Some(monitor)) = app.primary_monitor() {
                // 记录启动时的显示器 id，跨屏拖动时用于判断是否真正换屏
                *app.state::<LastMonitor>().0.lock().unwrap() =
                    format!("{}x{}", monitor.position().x, monitor.position().y);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_default_download_dir,
            get_app_version,
            downloader::check_system,
            downloader::parse_video,
            download::start_download,
            download::pause_download,
            download::cancel_download,
            download::delete_task,
            download::clear_task_part,
            cookies::cookie_store_path,
            cookies::cookie_store_status,
            cookies::add_cookie_store,
            cookies::remove_cookie_group,
            cookies::clear_cookie_store,
            history::delete_file,
            history::redownload_cleanup,
            history::load_tasks,
            history::save_tasks,
            history::register_task_output,
            history::load_history,
            history::save_history,
            install::install_ffmpeg,
            install::install_ytdlp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
