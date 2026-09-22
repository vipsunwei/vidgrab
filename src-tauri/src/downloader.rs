//! yt-dlp / ffmpeg 二进制查找与命令封装：搜索路径、可用性校验、JSON 抓取与解析命令。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use tauri::AppHandle;

use crate::cookies;
use crate::meta::{fetch_thumbnail_data_uri, parse_ytdlp_output, VideoInfo};
use crate::proc::hide_window;
use crate::url::clean_url;

// 跨平台二进制查找

/// 当前平台的 yt-dlp 可执行文件名（打包资源与运行时安装都用它）
pub fn ytdlp_binary_name() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

/// 当前平台的 ffmpeg 可执行文件名
pub fn ffmpeg_binary_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

/// 额外查找目录（resource_dir、app_data_dir/bin），由应用 setup 时注入。
/// 不能只用 exe 同目录：macOS 资源在 Contents/Resources 而可执行文件在
/// Contents/MacOS；Linux deb 的资源在 /usr/lib 下，均与 exe 不同级。
static EXTRA_SEARCH_DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();

pub fn set_search_dirs(dirs: Vec<PathBuf>) {
    let _ = EXTRA_SEARCH_DIRS.set(dirs);
}

fn extra_search_dirs() -> Vec<PathBuf> {
    EXTRA_SEARCH_DIRS.get().cloned().unwrap_or_default()
}

/// unix 下确保文件有可执行权限（tauri 打包资源/下载产物不保证保留 +x）
#[cfg(unix)]
fn ensure_exec(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.permissions().mode() & 0o111 == 0 {
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
        }
    }
}

#[cfg(not(unix))]
fn ensure_exec(_path: &Path) {}

/// 在目录列表中找名为 name 的可执行文件
fn find_in_dirs(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    for dir in dirs {
        let p = dir.join(name);
        if p.is_file() {
            ensure_exec(&p);
            return Some(p);
        }
    }
    None
}

/// 在 exe 同目录与注入的资源/数据目录中查找名为 name 的二进制
pub fn find_bundled_binary(name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(dir) = exe_dir.parent() {
            // Windows 安装版的 resources 通常在 exe 同目录，也可能保留 bin/ 结构
            dirs.push(dir.join("bin"));
            dirs.push(dir.to_path_buf());
        }
    }
    dirs.extend(extra_search_dirs());
    find_in_dirs(&dirs, name)
}

/// 查找 yt-dlp 可执行文件路径（返回第一个存在的候选）
///
/// 查找顺序：
/// 1. exe 同目录（Windows 安装版 / AppImage）
/// 2. 注入的资源/数据目录（macOS Contents/Resources、Linux deb 资源目录、运行时安装目录）
/// 3. 环境变量 YTDLP_PATH
/// 4. 常见安装位置
/// 5. PATH 查找
pub fn ytdlp_candidates() -> Vec<PathBuf> {
    let name = ytdlp_binary_name();
    if let Some(found) = find_bundled_binary(name) {
        return vec![found];
    }

    // 环境变量
    if let Ok(p) = std::env::var("YTDLP_PATH") {
        let path = PathBuf::from(p);
        if path.exists() {
            return vec![path];
        }
    }

    // 常见位置
    let candidates = [
        "C:/Program Files/yt-dlp/yt-dlp.exe",
        "C:/ProgramData/chocolatey/bin/yt-dlp.exe",
        "/usr/local/bin/yt-dlp",
        "/usr/bin/yt-dlp",
        "/opt/homebrew/bin/yt-dlp",
    ];
    let mut found: Vec<PathBuf> = candidates
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect();

    // PATH 查找
    if found.is_empty() {
        if let Ok(p) = which_ytdlp() {
            found.push(p);
        }
    }
    found
}

/// 查找 yt-dlp（不校验可运行；仅单元测试用，生产路径统一走 find_ytdlp_runner）
#[cfg(test)]
pub fn find_ytdlp() -> Option<PathBuf> {
    ytdlp_candidates().into_iter().next()
}

/// yt-dlp 的执行方式。
/// Windows/Linux：直接执行打包的独立二进制（PyInstaller onefile，自包含）；
/// macOS：捆绑的 python 解释器直跑 yt-dlp 源码包。
/// 背景：官方 yt-dlp_macos（PyInstaller onefile）在本机 macOS 上每次启动被系统
/// 阻塞约 40s（实测），触发 VidGrab 解析超时误杀与系统弹窗；捆绑 python 直跑
/// 源码包实测启动 <1s，且不依赖用户机器安装 python（yt-dlp 2026.08.19 要求
/// Python ≥3.10，macOS 系统自带 3.9 也跑不了，故捆绑 3.12）。
#[derive(Clone, Debug)]
pub struct YtdlpRunner {
    pub program: PathBuf,
    pub pre_args: Vec<String>,
    pub extra_env: Vec<(String, String)>,
}

impl YtdlpRunner {
    /// 构造执行命令（含 pre_args 与 extra_env 预设）
    pub fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        for a in &self.pre_args {
            cmd.arg(a);
        }
        for (k, v) in &self.extra_env {
            cmd.env(k, v);
        }
        cmd
    }
}

/// 候选查找目录：exe 同目录 + 注入的资源/数据目录（与 find_bundled_binary 一致）
/// 仅 macOS 的捆绑 python 策略使用（find_ytdlp_pkg_dir / macos_bundled_runner），
/// 不带 cfg 会在 Windows/Linux 编译时触发 dead_code。
#[cfg(target_os = "macos")]
fn all_search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(dir) = exe_dir.parent() {
            dirs.push(dir.join("bin"));
            dirs.push(dir.to_path_buf());
        }
    }
    dirs.extend(extra_search_dirs());
    dirs
}

/// 在候选目录里找 yt-dlp 源码包目录（含 yt_dlp/__main__.py 的 yt-dlp-pkg）
#[cfg(target_os = "macos")]
fn find_ytdlp_pkg_dir() -> Option<PathBuf> {
    for dir in all_search_dirs() {
        let pkg = dir.join("yt-dlp-pkg").join("yt_dlp");
        if pkg.join("__main__.py").is_file() {
            return Some(pkg.parent()?.to_path_buf());
        }
    }
    None
}

/// macOS：捆绑 python（python-build-standalone）+ 源码包 → python3 -m yt_dlp
#[cfg(target_os = "macos")]
fn macos_bundled_runner() -> Option<YtdlpRunner> {
    let pkg_dir = find_ytdlp_pkg_dir()?;
    for dir in all_search_dirs() {
        for py in ["python/bin/python3.12", "python/bin/python3", "python/bin/python"] {
            let py_path = dir.join(py);
            if py_path.is_file() {
                // 打包资源不保证保留 +x（tar/zip 解压工具可能丢权限），补一下再执行
                ensure_exec(&py_path);
                return Some(YtdlpRunner {
                    program: py_path,
                    pre_args: vec!["-m".into(), "yt_dlp".into()],
                    // PYTHONNOUSERSITE：隔离用户 site-packages，防止本机装的
                    // 其他 Python 包/yt-dlp 版本混入捆绑解释器（捆绑是自包含的）
                    extra_env: vec![
                        ("PYTHONPATH".into(), pkg_dir.to_string_lossy().into_owned()),
                        ("PYTHONNOUSERSITE".into(), "1".into()),
                    ],
                });
            }
        }
    }
    None
}

/// macOS：系统 python3（CLT shim 3.9）兜底 + 找到的源码包。
/// 注意：CLT 的 python3 是 3.9.6，跑不了 yt-dlp ≥2025.11（要求 ≥3.10）；
/// 仅当捆绑 python 缺失（异常构建）时兜底，且只在 yt-dlp 报 Python 版本不支持
/// 时才会被 verify 判为不可用，前端会引导走运行时安装。
#[cfg(target_os = "macos")]
fn macos_system_python_runner() -> Option<YtdlpRunner> {
    let pkg_dir = find_ytdlp_pkg_dir()?;
    let py = PathBuf::from("/usr/bin/python3");
    if py.is_file() {
        return Some(YtdlpRunner {
            program: py,
            pre_args: vec!["-m".into(), "yt_dlp".into()],
            // 系统 python 更要用 PYTHONNOUSERSITE：用户 pip --user 装的包
            // （如旧版 yt-dlp）不得污染源码包直跑路径
            extra_env: vec![
                ("PYTHONPATH".into(), pkg_dir.to_string_lossy().into_owned()),
                ("PYTHONNOUSERSITE".into(), "1".into()),
            ],
        });
    }
    None
}

/// 当前平台可用的 yt-dlp runner（解析/下载/校验统一入口）。
/// macOS 优先捆绑 python 直跑源码包；找不到再回退到 yt-dlp 独立二进制候选
/// （兼容旧包资源与外部安装）。Windows/Linux 走独立二进制。
pub fn ytdlp_runner_candidates() -> Vec<YtdlpRunner> {
    let mut runners: Vec<YtdlpRunner> = Vec::new();
    #[cfg(target_os = "macos")]
    {
        if let Some(r) = macos_bundled_runner() {
            runners.push(r);
        } else if let Some(r) = macos_system_python_runner() {
            runners.push(r);
        }
    }
    for p in ytdlp_candidates() {
        runners.push(YtdlpRunner {
            program: p,
            pre_args: vec![],
            extra_env: vec![],
        });
    }
    runners
}

/// 查找 yt-dlp runner（不校验可运行，解析/下载路径用）
pub fn find_ytdlp_runner() -> Option<YtdlpRunner> {
    ytdlp_runner_candidates().into_iter().next()
}

/// 校验 yt-dlp runner 真实可用（能跑通 --version）
pub fn verify_ytdlp_runner(runner: &YtdlpRunner) -> bool {
    let mut cmd = runner.build_command();
    cmd.arg("--version");
    hide_window(&mut cmd);
    matches!(
        cmd.output(),
        Ok(output) if output.status.success()
    )
}

/// 查找第一个真实可运行的 yt-dlp（check_system / 安装判断用）。
/// 能识别「文件存在但跑不起来」的坏二进制（如依赖系统 Python 的启动器存根）。
/// 返回 program 路径（兼容旧调用方；实际执行请用 runner）。
pub fn find_working_ytdlp() -> Option<PathBuf> {
    ytdlp_runner_candidates()
        .into_iter()
        .find(verify_ytdlp_runner)
        .map(|r| r.program)
}

/// 查找 ffmpeg 可执行文件路径。
/// 顺序：打包路径（exe 同目录 / 资源目录 / 运行时安装目录）优先，其次 PATH 与常见安装位置。
pub fn find_ffmpeg() -> Option<PathBuf> {
    if let Some(found) = find_bundled_binary(ffmpeg_binary_name()) {
        return Some(found);
    }
    let candidates = [
        "ffmpeg",
        "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
        "C:\\ProgramData\\chocolatey\\bin\\ffmpeg.exe",
        "/opt/homebrew/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/usr/bin/ffmpeg",
    ];
    for c in &candidates {
        let mut cmd = Command::new(c);
        cmd.arg("-version");
        hide_window(&mut cmd);
        if cmd.output().is_ok() {
            return Some(PathBuf::from(c));
        }
    }
    None
}

fn which_ytdlp() -> Result<PathBuf, ()> {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    let mut command = Command::new(cmd);
    command.arg("yt-dlp");
    hide_window(&mut command);
    let output = command.output().map_err(|_| ())?;
    if !output.status.success() {
        return Err(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().ok_or(())?;
    Ok(PathBuf::from(first_line.trim()))
}

/// 判断是否为可用的 Cookie 文件路径。
/// 以 .txt 结尾且该文件真实存在——非文件路径（含旧的浏览器名）一律忽略，
/// 避免把任意字符串当路径喂给 yt-dlp。
pub fn is_cookie_file(src: &str) -> bool {
    src.ends_with(".txt") && Path::new(src).is_file()
}

/// 把 cookie 参数追加到 yt-dlp 参数序列（解析与下载共用）。仅文件路径才追加 --cookies。
pub fn push_cookie_args(args: &mut Vec<String>, cookie: &str) {
    if is_cookie_file(cookie) {
        args.push("--cookies".into());
        args.push(cookie.to_string());
    }
}

/// 执行 yt-dlp --dump-json，拿到视频元数据 JSON（纯抓取，不下载）。
/// 便捷入口：不关心 PID 槽（无外部超时控制时使用）。集成测试依赖它。
#[allow(dead_code)]
pub fn run_ytdlp_dump(url: &str, cookie: Option<&str>) -> Result<String, String> {
    run_ytdlp_dump_with_pid(url, cookie, &std::sync::Mutex::new(None))
}

/// 带 PID 回传的解析执行：spawn 后立即把 PID 写入 pid_out，
/// 供调用方在超时时终止整棵进程树（避免僵尸解析进程）
pub fn run_ytdlp_dump_with_pid(
    url: &str,
    cookie: Option<&str>,
    pid_out: &std::sync::Mutex<Option<u32>>,
) -> Result<String, String> {
    let runner = find_ytdlp_runner().ok_or_else(|| {
        "未找到 yt-dlp（打包版本异常）".to_string()
    })?;

    let url = clean_url(url);
    // 防护：清洗后仍不是合法 http(s) 链接（如误把错误提示文本粘回输入框），
    // 直接友好报错，避免整段文本被当成 URL 喂给 yt-dlp 产生不可读嵌套错误
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("请输入有效的视频链接（需以 http:// 或 https:// 开头）".to_string());
    }

    let mut cmd = runner.build_command();
    // unix：设独立进程组（与下载路径一致），停止时 kill(-pid) 才能杀整组，
    // 否则解析进程与主程序同组，树杀落空、Python 子进程变孤儿残留（#1）
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.arg("--dump-json")
        .arg("--no-download")
        .arg("--no-warnings")
        .arg("--no-playlist")
        .arg("--socket-timeout")
        .arg("30");
    // yt-dlp 是 Python 程序：Windows 下 stdout 为管道时 Python 默认用本地编码
    // （中文系统 GBK），强制 UTF-8 保证输出可解析
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("PYTHONUTF8", "1");
    if let Some(src) = cookie {
        if !src.is_empty() {
            let mut cookie_args: Vec<String> = Vec::new();
            push_cookie_args(&mut cookie_args, src);
            for a in cookie_args {
                cmd.arg(a);
            }
        }
    }
    cmd.arg(&url);
    hide_window(&mut cmd);

    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 yt-dlp 失败: {}", e))?;
    *pid_out.lock().unwrap() = Some(child.id());

    let out = child
        .wait_with_output()
        .map_err(|e| format!("yt-dlp 进程出错: {}", e))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // 提取关键错误行，便于前端直接展示原因
        let hint = stderr
            .lines()
            .filter(|l| {
                let l = l.to_lowercase();
                l.contains("error") || l.contains("sign in") || l.contains("cookie") || l.contains("bot")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let detail = if hint.is_empty() {
            stderr.trim().to_string()
        } else {
            hint
        };
        return Err(format!("yt-dlp 解析失败：\n{}", detail));
    }

    // 宽松转换：正常情况为 UTF-8（已强制），坏字节以替换符呈现而不是整体失败
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// 依赖可用性：yt-dlp 按「能真实运行」判定，ffmpeg 按「能找到」判定。
#[derive(serde::Serialize)]
pub struct SystemStatus {
    pub yt_dlp: bool,
    pub ffmpeg: bool,
}

#[tauri::command]
pub async fn check_system() -> SystemStatus {
    // yt-dlp 不止查存在，还要能真实跑通 --version：
    // 防止打包进来的启动器存根（依赖系统 Python）在没装 Python 的机器上被误判为已安装
    SystemStatus {
        yt_dlp: find_working_ytdlp().is_some(),
        ffmpeg: find_ffmpeg().is_some(),
    }
}

#[tauri::command]
pub async fn parse_video(
    url: &str,
    cookie_source: Option<String>,
    app: AppHandle,
) -> Result<VideoInfo, String> {
    if url.is_empty() {
        return Err("URL 不能为空".to_string());
    }
    let url = url.to_string();
    // 现场复制 cookie 到运行时副本：yt-dlp 会写回 --cookies 文件，
    // 直接传用户存储会被污染（分组标记丢失、登录态被匿名 session 覆盖），
    // 见 cookies::prepare_cookie_runtime。
    let cookie = match &cookie_source {
        Some(src) if !src.is_empty() => Some(cookies::prepare_cookie_runtime(&app, src)?),
        _ => None,
    };
    let url_for_blocking = url.clone();
    // PID 槽：解析进程 spawn 后回传，超时时据此终止整棵进程树（避免僵尸解析进程）
    let pid_slot = std::sync::Arc::new(std::sync::Mutex::new(None::<u32>));
    let pid_slot_for_task = pid_slot.clone();
    // 在阻塞线程中执行同步的 yt-dlp 调用，并加 90s 超时
    // （PyInstaller onefile 版 yt-dlp 在本机 macOS 启动曾被系统阻塞 ~40s，
    //  换捆绑 python 直跑后 <1s；90s 仍保留为异常网络/站点慢的兜底）
    let parse_fut = tokio::task::spawn_blocking(move || {
        run_ytdlp_dump_with_pid(&url_for_blocking, cookie.as_deref(), &pid_slot_for_task)
    });
    match tokio::time::timeout(tokio::time::Duration::from_secs(90), parse_fut).await {
        Err(_) => {
            // 超时：终止仍在运行的解析进程树（先取出 PID 并释放锁，再跨 await 杀树）
            let pid = pid_slot.lock().unwrap().take();
            if let Some(pid) = pid {
                crate::proc::kill_pid_tree(pid).await;
            }
            Err("解析超时（90s），已终止解析进程".to_string())
        }
        Ok(joined) => {
            let json = joined.map_err(|e| format!("yt-dlp 调用失败: {}", e))??;
            let mut info = parse_ytdlp_output(&json, &url)?;
            // 把缩略图下载成 data URI，规避 WebView 加载外链图片的限制
            if !info.thumbnail.is_empty() {
                let data_uri = fetch_thumbnail_data_uri(&info.thumbnail).await;
                if !data_uri.is_empty() {
                    info.thumbnail = data_uri;
                }
            }
            Ok(info)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "需网络 + yt-dlp，本地手动跑验证"]
    fn integration_run_ytdlp_dump_real_url() {
        let url = "https://www.youtube.com/watch?v=3Va-3carIcA";
        let json = run_ytdlp_dump(url, None).expect("yt-dlp 需可执行");
        assert!(!json.is_empty());
        let info = parse_ytdlp_output(&json, url).expect("应能解析");
        assert!(info.title.contains("神曲"));
        assert!(!info.formats.is_empty());
        println!("真实下载信息: title={}, formats={}", info.title, info.formats.len());
    }

    #[test]
    #[ignore = "需 yt-dlp 存在"]
    fn integration_find_ytdlp() {
        let path = find_ytdlp();
        assert!(path.is_some(), "应能找到 yt-dlp");
        println!("yt-dlp 路径: {:?}", path.unwrap());
    }

    #[test]
    fn is_cookie_file_requires_txt_and_existing() {
        let dir = std::env::temp_dir().join(format!("vidgrab_cookie_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let txt = dir.join("cookies.txt");
        std::fs::write(&txt, "# Netscape cookie\n").unwrap();
        let not_txt = dir.join("cookies");
        std::fs::write(&not_txt, "x").unwrap();

        // .txt 且真实存在 → true
        assert!(is_cookie_file(txt.to_str().unwrap()));
        // 非 .txt 后缀 → false
        assert!(!is_cookie_file(not_txt.to_str().unwrap()));
        // 不存在的路径 → false
        assert!(!is_cookie_file("C:\\no\\such\\file.txt"));
        // 浏览器名 / 空串 / "none" → false（不是文件路径）
        assert!(!is_cookie_file(""));
        assert!(!is_cookie_file("none"));
        assert!(!is_cookie_file("chrome"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn push_cookie_args_only_for_file() {
        let dir = std::env::temp_dir().join(format!("vidgrab_pushcookie_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let txt = dir.join("c.txt");
        std::fs::write(&txt, "x").unwrap();

        let mut args = vec!["--no-warnings".into()];
        push_cookie_args(&mut args, txt.to_str().unwrap());
        assert_eq!(args, vec!["--no-warnings".to_string(), "--cookies".to_string(), txt.to_str().unwrap().to_string()]);

        // 非文件路径 → 不追加任何参数
        let mut args2 = vec!["--no-warnings".into()];
        push_cookie_args(&mut args2, "none");
        push_cookie_args(&mut args2, "");
        push_cookie_args(&mut args2, "firefox");
        assert_eq!(args2, vec!["--no-warnings".to_string()]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
