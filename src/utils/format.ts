// 纯展示格式化与轻量解析函数。无副作用、无框架依赖，便于单测与复用。
import type { FormatInfo } from '@/types'

export function formatSize(bytes: number | null | undefined): string {
  if (!bytes || bytes <= 0) return ''
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(0)} MB`
  return `${(bytes / 1_000).toFixed(0)} KB`
}

export function formatHistoryTime(ts: number): string {
  const d = new Date(ts)
  const now = new Date()
  const isToday = d.toDateString() === now.toDateString()
  const timeStr = d.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
  })
  if (isToday) return `今天 ${timeStr}`
  return `${d.getMonth() + 1}/${d.getDate()} ${timeStr}`
}

export function isToday(ts: number): boolean {
  return new Date(ts).toDateString() === new Date().toDateString()
}

/// 秒 → "H:MM:SS" / "M:SS"；0 或空显示占位符
export function formatDuration(seconds: number): string {
  if (!seconds) return '--:--'
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = seconds % 60
  return h > 0
    ? `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
    : `${m}:${s.toString().padStart(2, '0')}`
}

/// 从 "1920x1080" 取高度；非标准值（如 "audio only"）返回 0
export function parseHeight(res: string): number {
  const match = res.match(/(\d+)\s*x\s*(\d+)/)
  if (match) return parseInt(match[2], 10) || 0
  return parseInt(res, 10) || 0
}

/// 从 URL 推断平台名（与后端 url::detect_platform 保持一致）。
/// 无法识别时返回空串，由调用方兜底。
export function detectPlatform(raw: string): string {
  const u = (raw || '').toLowerCase()
  if (u.includes('bilibili.com') || u.includes('b23.tv')) return '哔哩哔哩'
  if (u.includes('youtube.com') || u.includes('youtu.be')) return 'YouTube'
  if (u.includes('douyin.com')) return '抖音'
  if (u.includes('xiaohongshu.com') || u.includes('xhslink.com')) return '小红书'
  if (u.includes('weibo.com')) return '微博'
  if (u.includes('v.qq.com')) return '腾讯视频'
  if (u.includes('iqiyi.com')) return '爱奇艺'
  if (u.includes('youku.com')) return '优酷'
  if (u.includes('douyu.com')) return '斗鱼'
  if (u.includes('huya.com')) return '虎牙'
  if (u.includes('acfun.cn')) return 'AcFun'
  if (u.includes('twitter.com') || u.includes('x.com')) return 'Twitter / X'
  if (u.includes('instagram.com')) return 'Instagram'
  if (u.includes('facebook.com') || u.includes('fb.watch')) return 'Facebook'
  return ''
}

/// 格式体积标签：精确值优先，其次估算值（带 ~ 前缀），都没有则返回空
export function formatSizeLabel(f: FormatInfo): string {
  if (f.filesize) return `· ${formatSize(f.filesize)}`
  if (f.filesize_approx) return `· ~${formatSize(f.filesize_approx)}`
  return ''
}
