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

/// 各平台的 ffmpeg 下载源 (url, 压缩包文件名)。
/// 与构建期 scripts/fetch-binaries.ts 同款 eugeneware/ffmpeg-static 固定版本单文件 .gz，
/// 以便运行时校验的 SHA256 与打包二进制一致（构建期已算好注入）。
pub fn ffmpeg_sources() -> Vec<(String, String)> {
    let ver = match option_env!("VIDGRAB_FFMPEG_VERSION") {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => return vec![], // 构建信息缺失则不下（避免无校验下载）
    };
    let base = format!("https://github.com/eugeneware/ffmpeg-static/releases/download/{ver}");
    if cfg!(target_os = "windows") {
        vec![(format!("{base}/ffmpeg-win32-x64.gz"), "ffmpeg-win32-x64.gz".to_string())]
    } else if cfg!(target_os = "macos") {
        let asset = if cfg!(target_arch = "aarch64") {
            "ffmpeg-darwin-arm64"
        } else {
            "ffmpeg-darwin-x64"
        };
        vec![(format!("{base}/{asset}.gz"), format!("{asset}.gz"))]
    } else {
        let asset = if cfg!(target_arch = "aarch64") {
            "ffmpeg-linux-arm64"
        } else {
            "ffmpeg-linux-x64"
        };
        vec![(format!("{base}/{asset}.gz"), format!("{asset}.gz"))]
    }
}


/// 解压 .gz 单文件到 dest（eugeneware/ffmpeg-static 的资产是 gzip 压缩的单文件）
pub fn extract_gz_archive(gz_path: &Path, dest: &Path) -> Result<(), String> {
    let f = std::fs::File::open(gz_path).map_err(|e| format!("读取压缩包失败: {}", e))?;
    let mut decoder = flate2::read::GzDecoder::new(f);
    let mut out = std::fs::File::create(dest).map_err(|e| format!("创建解压文件失败: {}", e))?;
    std::io::copy(&mut decoder, &mut out).map_err(|e| format!("解压失败: {}", e))?;
    Ok(())
}

/// 计算文件 SHA256（小写十六进制），供安装器校验二进制完整性
fn sha256_of(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).map_err(|e| format!("读取文件失败: {}", e))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| format!("计算哈希失败: {}", e))?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// 当前平台期望的 ffmpeg SHA256（构建期由 fetch-binaries 算好注入；缺则 None 跳过校验）
fn expected_ffmpeg_sha() -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            option_env!("VIDGRAB_FFMPEG_DARWIN_ARM64_SHA")
        } else {
            option_env!("VIDGRAB_FFMPEG_DARWIN_X64_SHA")
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            option_env!("VIDGRAB_FFMPEG_LINUX_ARM64_SHA")
        } else {
            option_env!("VIDGRAB_FFMPEG_LINUX_X64_SHA")
        }
    } else if cfg!(target_os = "windows") {
        option_env!("VIDGRAB_FFMPEG_WIN32_X64_SHA")
    } else {
        None
    }
}

/// 当前平台期望的 yt-dlp SHA256
fn expected_ytdlp_sha() -> Option<&'static str> {
    if cfg!(target_os = "windows") {
        option_env!("VIDGRAB_YTDLP_WIN_SHA")
    } else if cfg!(target_os = "macos") {
        option_env!("VIDGRAB_YTDLP_MAC_SHA")
    } else {
        option_env!("VIDGRAB_YTDLP_LINUX_SHA")
    }
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
    if sources.is_empty() {
        return Err("构建信息缺失（未注入 ffmpeg 版本/哈希），无法安全下载安装".to_string());
    }
    let (url, file_name) = &sources[0];
    let dst_dir = bin_install_dir(&app)?;

    let tmp_dir = std::env::temp_dir().join("vidgrab-ffmpeg-install");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    // 下载 .gz 单文件
    let total_bytes = fetch_content_length(url).await.unwrap_or(80_000_000_u64);
    let response = client.get(url).send().await
        .map_err(|e| format!("下载 FFmpeg 失败: {}", e))?;
    let gz_path = tmp_dir.join(file_name);
    let mut file = std::fs::File::create(&gz_path)
        .map_err(|e| format!("创建下载文件失败: {}", e))?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    while let Some(chunk) = futures_util::stream::StreamExt::next(&mut stream).await {
        let chunk = chunk.map_err(|e| format!("下载流失败: {}", e))?;
        downloaded += chunk.len() as u64;
        let overall = if total_bytes > 0 {
            (downloaded as f64 / total_bytes as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
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
    std::io::Write::flush(&mut file).map_err(|e| format!("刷新文件失败: {}", e))?;

    let _ = app.emit("ffmpeg-install-progress", FFmpegInstallProgress {
        progress: 100.0,
        speed: "完成".to_string(),
        downloaded_mb: 0.0,
        total_mb: 0.0,
        stage: "extracting".to_string(),
    });

    // 解压 .gz 单文件得到 ffmpeg，复制到安装目录前先做 SHA256 校验
    let ffmpeg_name = ffmpeg_binary_name();
    let ffmpeg_tmp = tmp_dir.join(ffmpeg_name);
    extract_gz_archive(&gz_path, &ffmpeg_tmp)?;
    if let Some(exp) = expected_ffmpeg_sha() {
        let actual = sha256_of(&ffmpeg_tmp)?;
        if actual.to_lowercase() != exp.to_lowercase() {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(format!(
                "ffmpeg 校验失败：哈希不匹配（期望 {exp}，实际 {actual}），可能下载被篡改"
            ));
        }
    }
    let ffmpeg_dst = dst_dir.join(ffmpeg_name);
    std::fs::copy(&ffmpeg_tmp, &ffmpeg_dst)
        .map_err(|e| format!("复制 ffmpeg 失败: {}", e))?;
    chmod_exec(&ffmpeg_dst)?;

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
/// 使用构建期锁定的固定版本（与打包二进制一致），并由构建期算好的 SHA256 运行时校验；
/// 构建信息缺失时降级为 latest（不校验，仅异常构建才会走到）。
pub fn get_ytdlp_url_for_platform() -> Option<(String, String)> {
    let ver = option_env!("VIDGRAB_YTDLP_VERSION").unwrap_or("latest");
    let base = if ver == "latest" {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download".to_string()
    } else {
        format!("https://github.com/yt-dlp/yt-dlp/releases/download/{ver}")
    };
    if cfg!(target_os = "windows") {
        Some((format!("{base}/yt-dlp.exe"), "yt-dlp.exe".to_string()))
    } else if cfg!(target_os = "macos") {
        Some((format!("{base}/yt-dlp_macos"), "yt-dlp".to_string()))
    } else {
        // 注意用独立版 yt-dlp_linux（PyInstaller 打包，无需 Python）；
        // release 里的 `yt-dlp` 资产是 python zipapp，裸机器跑不起来
        Some((format!("{base}/yt-dlp_linux"), "yt-dlp".to_string()))
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

    // SHA256 校验，防传输层篡改/损坏（校验值缺失时跳过，降级为不校验）
    if let Some(exp) = expected_ytdlp_sha() {
        let actual = sha256_of(&tmp_path)?;
        if actual.to_lowercase() != exp.to_lowercase() {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(format!(
                "yt-dlp 校验失败：哈希不匹配（期望 {exp}，实际 {actual}），可能下载被篡改"
            ));
        }
    }

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