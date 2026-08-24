//! 共享状态定义：任务表（PID）、取消/暂停标记（运行代号隔离）、输出位置注册表。
//! 拆分自 lib.rs，见各类型注释。
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;


/// 记录窗口当前所在显示器 id，跨屏拖动时仅在新屏幕变化时才重新适配大小，避免单屏内拖动抖动。
pub struct LastMonitor(pub Mutex<String>);

// 运行中任务表：task_id -> 子进程 PID。
// 只登记 PID 供停止时树杀；子进程对象由下载任务独占（绝不共享互斥锁——
// 曾因 runner 持锁 wait 导致暂停/取消命令阻塞在锁上，整段下载期间停不住）。
pub type TaskTable = Arc<AsyncMutex<HashMap<String, Option<u32>>>>;

// 取消中的 task_id 集合：进程被 kill 后据此区分「用户取消」与「真实失败」。
// 值为运行代号：只匹配自己那次启动，防止旧运行的清理误删新运行的标记
#[derive(Default)]
pub struct CancelSet(pub Arc<AsyncMutex<HashMap<String, u64>>>);
// 暂停请求集合：pause_download 插入，任务结束时据此区分「用户暂停」并清理。
// 暂停 = 终止子进程但保留 .part 断点；继续 = 前端用原参数重新 start_download，
// yt-dlp 检测到同名 .part 自动断点续传。
#[derive(Default)]
pub struct PausingSet(pub Arc<AsyncMutex<HashMap<String, u64>>>);
// 任务最后一次启动的运行代号：task_id -> seq。
// 每次启动分配递增序号；事件只回给最后一次启动的运行——快速「取消→重新下载」
// 时，旧运行的迟到事件不得覆盖新运行的状态。
#[derive(Default)]
pub struct TaskSeqs(pub Arc<Mutex<HashMap<String, u64>>>);
// 运行代号发生器
static RUN_SEQ: AtomicU64 = AtomicU64::new(0);
// 任务输出位置注册表：task_id -> (输出目录, 文件名前缀)，删除任务时用于清理 .part 残留
#[derive(Default)]
pub struct TaskOutputs(pub Arc<Mutex<HashMap<String, (PathBuf, String)>>>);


/// 分配下一个运行代号（每次 start_download 调用一次）
pub fn next_run_seq() -> u64 {
    RUN_SEQ.fetch_add(1, Ordering::Relaxed) + 1
}

/// 判断「代号为 seq 的这次运行」是否被用户暂停。
/// 严格比对代号而非只看 key 是否存在：快速「取消 → 重新下载」时，
/// 残留的旧标记不得让新运行一启动就自己退出。
pub async fn is_paused(pausing: &PausingSet, task_id: &str, seq: u64) -> bool {
    pausing.0.lock().await.get(task_id).copied() == Some(seq)
}

/// 判断「代号为 seq 的这次运行」是否被用户取消（同上，按代号严格匹配）。
pub async fn is_cancelled(cancelling: &CancelSet, task_id: &str, seq: u64) -> bool {
    cancelling.0.lock().await.get(task_id).copied() == Some(seq)
}