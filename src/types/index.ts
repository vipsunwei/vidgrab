// 前端共享类型：与 Rust 侧的 serde 结构一一对应，避免各组件各写一份。

/// yt-dlp 解析出的单条可用格式
export interface FormatInfo {
  format_id: string
  ext: string
  resolution: string
  filesize: number | null
  filesize_approx: number | null
  vcodec: string
  acodec: string
  format_note: string
  is_audio_only: boolean
  /// 音频码率（kbps）
  abr: number | null
  /// 音频采样率（Hz）
  asr: number | null
  /// 音频声道数
  audio_channels: number | null
}


/// parse_video 命令的返回结构
export interface VideoMetadata {
  title: string
  thumbnail: string
  uploader: string
  duration: number
  platform: string
  formats: FormatInfo[]
}

/// 一条下载历史记录（持久化到 history.json）
export interface DownloadRecord {
  id: string
  url: string
  title: string
  platform: string
  thumbnail: string
  outputPath: string
  taskId: string
  createdAt: number
  size: number | null
  /// 画质标签（如 1080p / 320k），用于「同视频同画质」去重，避免无提示重复下载
  qualityTag: string
  /// 下载目录快照（重下载时复用，不受设置变动影响）
  outputDir: string
  /// 用户自定义文件名（无则为空，由视频标题派生），重下载时复用以保证命中同一文件名
  outputName: string
  /// 下载模式：merge=视频+音频合并 / audio=仅音频 / video=仅视频(含音轨)
  mode: DownloadMode
  /// yt-dlp 格式 ID 列表（单轨 1 个，双轨 [视频, 音频]），重下载时原样传回后端
  formatIds: string[]
}

/// 下载模式：merge=视频+音频合并 / audio=仅音频 / video=仅视频(含音轨)
export type DownloadMode = 'merge' | 'audio' | 'video'

/// 后端上报的阶段
export type TaskStage = 'video' | 'audio' | 'merge' | 'done' | 'idle' | 'queued'

/// 任务在前端视角的状态
export type TaskStatus =
  | 'queued'
  | 'downloading'
  | 'done'
  | 'error'
  | 'cancelled'
  | 'paused'

/// 一个并行下载任务的完整运行态
export interface DownloadTask {
  id: string
  url: string
  title: string
  thumbnail: string
  mode: DownloadMode
  formatIds: string[]
  stage: TaskStage
  status: TaskStatus
  videoProgress: number
  audioProgress: number
  mergeProgress: number
  speed: string
  eta: string
  outputPath: string
  error: string
  videoSize: number | null
  audioSize: number | null
  outputName: string
  /// 下载目录快照：暂停任务持久化后重启恢复/继续下载时使用，不受设置变动影响
  outputDir: string
  /// 质量标签（1080p / 160k / audio…）：同视频多清晰度并行下载的文件名区分与防重复判定
  qualityTag: string
}

/// 左侧导航的四个视图
export type NavKey = 'parse' | 'downloading' | 'history' | 'settings'

/// 轨道胶囊的展示数据（由 FormatInfo 派生，供 UI 直接渲染）
export interface TrackCapsule {
  key: string
  label: string
  sub: string
  size: string
}
