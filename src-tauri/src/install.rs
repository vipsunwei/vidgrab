//! 运行时安装器：ffmpeg / yt-dlp 缺失时的下载、解压与自修复命令。
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::downloader::{ffmpeg_binary_name, find_ffmpeg, find_working_ytdlp};


#[derive(Serialize)]
#[derive(Clone)]
pub struct FFmpegInstallProgress {
    progress: f64,
    speed: String,
    downloaded_mb: f64,
    total_mb: f64,
    stage: String, // "downloading" | "extracting" | "done"
}

/// 各平台的 ffmpeg 下载源列表 (url, 保存文件名)。
/// - Windows: BtbN win64 zip（bin/ 目录内含 ffmpeg、ffprobe）
/// - macOS: BtbN 不提供 macOS 构建，用 evermeet.cx 的独立二进制 zip（ffmpeg、ffprobe 各一个）
/// - Linux: BtbN linux64 tar.xz
pub fn ffmpeg_sources() -> Vec<(String, String)> {
    if cfg!(target_os = "windows") {
        vec![(
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip".to_string(),
            "ffmpeg-win64.zip".to_string(),
        )]
    } else if cfg!(target_os = "macos") {
        vec![
            (
                "https://evermeet.cx/ffmpeg/getrelease/zip".to_string(),
                "ffmpeg-macos.zip".to_string(),
            ),
            (
                "https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip".to_string(),
                "ffprobe-macos.zip".to_string(),
            ),
        ]
    } else {
        vec![(
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz".to_string(),
            "ffmpeg-linux64.tar.xz".to_string(),
        )]
    }
}


/// 解压 zip 到 dest（全平台）
pub fn extract_zip_archive(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let zip_data =
        std::fs::read(zip_path).map_err(|e| format!("读取压缩包失败: {}", e))?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&zip_data))
        .map_err(|e| format!("解压失败: {}", e))?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("读取压缩包内文件失败: {}", e))?;
        let outpath = dest.join(file.mangled_name());
        if file.is_dir() {
            std::fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| format!("创建解压文件失败: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("写入解压文件失败: {}", e))?;
        }
    }
    Ok(())
}


/// 解压 tar.xz 到 dest（Linux 的 BtbN 包是 tar.xz 格式）
#[cfg(target_os = "linux")]
pub fn extract_tar_xz_archive(path: &Path, dest: &Path) -> Result<(), String> {
    let f = std::fs::File::open(path).map_err(|e| format!("读取压缩包失败: {}", e))?;
    let decoder = xz2::read::XzDecoder::new(f);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest).map_err(|e| format!("解压失败: {}", e))
}

#[cfg(not(target_os = "linux"))]
pub fn extract_tar_xz_archive(_path: &Path, _dest: &Path) -> Result<(), String> {
    Err("当前平台不使用 tar.xz 包".to_string())
}


/// 在目录中递归查找文件名精确匹配的文件（压缩包层级浅，无需剪枝）
pub fn find_file_recursive(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_file() {
            if p.file_name().map(|n| n == name).unwrap_or(false) {
                return Some(p);
            }
        } else if p.is_dir() {
            if let Some(found) = find_file_recursive(&p, name) {
                return Some(found);
            }
        }
    }
    None
}


/// 从 HTTP Content-Length 获取总大小（字节），失败返回 None
pub async fn fetch_content_length(url: &str) -> Option<u64> {
    let resp = reqwest::get(url).await.ok()?;
    resp.content_length()
}


#[tauri::command]
pub async fn install_ffmpeg(app: tauri::AppHandle) -> Result<String, String> {
    // 已有 ffmpeg 则跳过
    if find_ffmpeg().is_some() {
        return Ok("already_installed".to_string());
    }

    let sources = ffmpeg_sources();
    let dst_dir = bin_install_dir(&app)?;

    let tmp_dir = std::env::temp_dir().join("vidgrab-ffmpeg-install");
    let extract_dir = tmp_dir.join("extract");

    // 确保临时目录干净
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    // 逐个下载源文件，进度按 (已完成文件数 + 当前文件比例) / 总数 折算
    for (idx, (url, file_name)) in sources.iter().enumerate() {
        let total_bytes = fetch_content_length(url).await
            .unwrap_or(200_000_000_u64); // fallback 200MB

        let response = client.get(url)
            .send()
            .await
            .map_err(|e| format!("下载 FFmpeg 失败: {}", e))?;

        let archive_path = tmp_dir.join(file_name);
        let mut file = std::fs::File::create(&archive_path)
            .map_err(|e| format!("创建下载文件失败: {}", e))?;
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = futures_util::stream::StreamExt::next(&mut stream).await
        {
            let chunk = chunk.map_err(|e| format!("下载流失败: {}", e))?;
            downloaded += chunk.len() as u64;
            let file_frac = if total_bytes > 0 {
                downloaded as f64 / total_bytes as f64
            } else {
                0.0
            };
            let overall = ((idx as f64 + file_frac) / sources.len() as f64 * 100.0).min(100.0);
            let _ = app.emit("ffmpeg-install-progress", FFmpegInstallProgress {
                progress: overall,
                speed: format!("{:.0} MB", downloaded as f64 / 1_048_576.0),
                downloaded_mb: downloaded as f64 / 1_048_576.0,
                total_mb: total_bytes as f64 / 1_048_576.0,
                stage: "downloading".to_string(),
            });
            std::io::Write::write_all(&mut file, &chunk)
                .map_err(|e| format!("写入文件失败: {}", e))?;
        }
        std::io::Write::flush(&mut file)
            .map_err(|e| format!("刷新文件失败: {}", e))?;
    }

    let _ = app.emit("ffmpeg-install-progress", FFmpegInstallProgress {
        progress: 100.0,
        speed: "完成".to_string(),
        downloaded_mb: 0.0,
        total_mb: 0.0,
        stage: "extracting".to_string(),
    });

    // 全部解压到同一目录
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("创建解压目录失败: {}", e))?;
    for (_url, file_name) in &sources {
        let archive_path = tmp_dir.join(file_name);
        if file_name.ends_with(".zip") {
            extract_zip_archive(&archive_path, &extract_dir)?;
        } else {
            extract_tar_xz_archive(&archive_path, &extract_dir)?;
        }
    }

    // 在解压目录中定位 ffmpeg / ffprobe，复制到安装目录
    // （ffprobe 目前代码未直接使用，找不到不阻塞安装）
    let ffmpeg_name = ffmpeg_binary_name();
    let ffprobe_name = if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" };
    let ffmpeg_src = find_file_recursive(&extract_dir, ffmpeg_name)
        .ok_or("解压后未找到 ffmpeg")?;
    let ffmpeg_dst = dst_dir.join(ffmpeg_name);
    std::fs::copy(&ffmpeg_src, &ffmpeg_dst)
        .map_err(|e| format!("复制 ffmpeg 失败: {}", e))?;
    chmod_exec(&ffmpeg_dst)?;
    if let Some(ffprobe_src) = find_file_recursive(&extract_dir, ffprobe_name) {
        let ffprobe_dst = dst_dir.join(ffprobe_name);
        std::fs::copy(&ffprobe_src, &ffprobe_dst)
            .map_err(|e| format!("复制 ffprobe 失败: {}", e))?;
        chmod_exec(&ffprobe_dst)?;
    }

    // 清理临时目录
    let _ = std::fs::remove_dir_all(&tmp_dir);

    let _ = app.emit("ffmpeg-install-progress", FFmpegInstallProgress {
        progress: 100.0,
        speed: "".to_string(),
        downloaded_mb: 0.0,
        total_mb: 0.0,
        stage: "done".to_string(),
    });

    Ok(format!("已安装到 {}", dst_dir.display()))
}

/// 运行时安装二进制（yt-dlp / ffmpeg）的目标目录。
/// Windows 用 exe 同目录（NSIS currentUser 安装，可写）；
/// macOS/Linux 的 exe 目录可能只读（/Applications、/usr/bin、AppImage 挂载点），用 app data 目录。
pub fn bin_install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    if cfg!(windows) {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .ok_or_else(|| "无法获取程序所在目录".to_string())
    } else {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("获取应用数据目录失败: {}", e))?
            .join("bin");
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建安装目录失败: {}", e))?;
        Ok(dir)
    }
}


/// unix 下给文件加可执行权限
#[cfg(unix)]
pub fn chmod_exec(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("设置可执行权限失败: {}", e))
}

#[cfg(not(unix))]
pub fn chmod_exec(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// 按平台返回 yt-dlp 的下载地址与目标文件名，即 (download_url, target_filename_with_ext)。
pub fn get_ytdlp_url_for_platform() -> Option<(String, String)> {
    if cfg!(target_os = "windows") {
        Some((
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe".to_string(),
            "yt-dlp.exe".to_string(),
        ))
    } else if cfg!(target_os = "macos") {
        Some((
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos".to_string(),
            "yt-dlp".to_string(),
        ))
    } else {
        // 注意用独立版 yt-dlp_linux（PyInstaller 打包，无需 Python）；
        // release 里的 `yt-dlp` 资产是 python zipapp，裸机器跑不起来
        Some((
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux".to_string(),
            "yt-dlp".to_string(),
        ))
    }
}


#[tauri::command]
pub async fn install_ytdlp(app: tauri::AppHandle) -> Result<String, String> {
    // 已存在且能真实运行才跳过；存在但跑不起来（如启动器存根）则继续重新下载修复
    if find_working_ytdlp().is_some() {
        return Ok("already_installed".to_string());
    }

    let Some((url, target_name)) = get_ytdlp_url_for_platform() else {
        return Err("不支持的操作系统".to_string());
    };

    let dst_dir = bin_install_dir(&app)?;
    let dst = dst_dir.join(&target_name);

    let tmp_dir = std::env::temp_dir().join("vidgrab-ytdlp-install");
    let tmp_path = tmp_dir.join(&target_name);
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;

    // 下载，推送进度
    let total_bytes = fetch_content_length(&url).await.unwrap_or(15_000_000u64); // fallback 15MB
    let total_mb = total_bytes as f64 / 1_048_576.0;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("下载 yt-dlp 失败: {}", e))?;

    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("创建下载文件失败: {}", e))?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    while let Some(chunk) =
        futures_util::stream::StreamExt::next(&mut stream).await
    {
        let chunk = chunk.map_err(|e| format!("下载流失败: {}", e))?;
        downloaded += chunk.len() as u64;
        let progress = (downloaded as f64 / total_bytes as f64 * 100.0).min(100.0);
        let _ = app.emit(
            "ytdlp-install-progress",
            FFmpegInstallProgress {
                progress,
                speed: format!("{:.1} MB", downloaded as f64 / 1_048_576.0),
                downloaded_mb: downloaded as f64 / 1_048_576.0,
                total_mb,
                stage: "downloading".to_string(),
            },
        );
        std::io::Write::write_all(&mut file, &chunk)
            .map_err(|e| format!("写入文件失败: {}", e))?;
    }
    std::io::Write::flush(&mut file).map_err(|e| format!("刷新文件失败: {}", e))?;

    // 移动到 exe 同目录
    std::fs::copy(&tmp_path, &dst).map_err(|e| format!("复制文件失败: {}", e))?;
    let _ = std::fs::remove_dir_all(&tmp_dir);

    // macOS / Linux 需要可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o755));
    }

    let _ = app.emit(
        "ytdlp-install-progress",
        FFmpegInstallProgress {
            progress: 100.0,
            speed: "".to_string(),
            downloaded_mb: total_mb,
            total_mb,
            stage: "done".to_string(),
        },
    );

    Ok(format!("已安装到 {}", dst.display()))
}