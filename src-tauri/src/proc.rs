//! 进程控制原语：树杀、KILL_ON_JOB_CLOSE 作业对象、创建标志、管道行读取。
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

use crate::state::TaskTable;

/// Windows 下抑制子进程控制台窗口的创建标志。
/// GUI 程序（release 版无控制台）spawn 控制台程序时，系统会新建一个黑色
/// cmd 窗口；加此标志后子进程静默运行。非 Windows 平台不适用。
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 给 std Command 统一加上 CREATE_NO_WINDOW（仅 Windows），避免 spawn 控制台程序时弹出黑框
pub fn hide_window(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// 给 tokio Command 统一加上 CREATE_NO_WINDOW（仅 Windows），避免 spawn 控制台程序时弹出黑框
pub fn hide_window_tokio(cmd: &mut Command) {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// 用户主动的暂停/取消仍走 taskkill 树杀，两条路径互不依赖。
#[cfg(windows)]
pub fn attach_kill_on_close_job(child: &tokio::process::Child) {
    let mut info = win32job::ExtendedLimitInfo::new();
    info.limit_kill_on_job_close();
    let Ok(job) = win32job::Job::create_with_limit_info(&info) else {
        return;
    };
    if let Some(raw) = child.raw_handle() {
        if job.assign_process(raw as isize).is_ok() {
            std::mem::forget(job);
        }
    }
}


/// 按字节读一行并宽松转 UTF-8。
/// yt-dlp/ffmpeg 在 Windows 管道下可能输出本地编码（GBK）字节：若按 UTF-8 严格
/// 解码，读取任务会因 InvalidData 意外退出并关闭管道，子进程下次写进度即得到
/// Windows 断管道错误（errno 22），表现为「下载失败 [Errno 22] Invalid argument」。
pub async fn read_line_lossy<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    buf: &mut Vec<u8>,
) -> tokio::io::Result<Option<String>> {
    buf.clear();
    let n = reader.read_until(b'\n', buf).await?;
    if n == 0 {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(buf).into_owned()))
}

/// 强制 Python 系子进程（yt-dlp）按 UTF-8 写 stdout/stderr。
/// Windows 下 stdout 为管道时 Python 默认用本地编码（中文系统 = GBK）。
pub fn force_utf8_env(cmd: &mut Command) {
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("PYTHONUTF8", "1");
}

/// 按 PID 终止整棵进程树（Windows taskkill /T；unix 进组 SIGKILL）
pub async fn kill_pid_tree(pid: u32) {
    #[cfg(windows)]
    {
        let mut killer = tokio::process::Command::new("taskkill");
        killer.args(["/F", "/T", "/PID", &pid.to_string()]);
        hide_window_tokio(&mut killer);
        let _ = killer.output().await;
    }
    #[cfg(not(windows))]
    {
        unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
    }
}

/// 终止任务进程（整棵进程树）。
/// yt-dlp.exe 是 PyInstaller 单文件打包：运行时为「引导器父进程 + Python 子进程」，
/// 只 kill 直接子进程会留下 Python 孤儿继续写 .part——双写损坏与「暂停停不住」的根因。
/// 只读 PID 表，绝不触碰子进程互斥锁（runner 在 wait 期间持有它，抢锁会死等）。
pub async fn stop_task(tasks: &TaskTable, task_id: &str) {
    let pid = tasks.lock().await.get(task_id).copied().flatten();
    let Some(pid) = pid else { return };
    kill_pid_tree(pid).await;
}