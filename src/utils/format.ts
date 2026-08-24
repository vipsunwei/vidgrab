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

/// 从 URL 推断平台名；仅识别已验证支持的两个平台，其余返回空串由调用方兜底
export function detectPlatform(raw: string): string {
  const u = (raw || '').toLowerCase()
  if (u.includes('bilibili.com') || u.includes('b23.tv')) return '哔哩哔哩'
  if (u.includes('youtube.com') || u.includes('youtu.be')) return 'YouTube'
  return ''
}

/// 格式体积标签：精确值优先，其次估算值（带 ~ 前缀），都没有则返回空
export function formatSizeLabel(f: FormatInfo): string {
  if (f.filesize) return `· ${formatSize(f.filesize)}`
  if (f.filesize_approx) return `· ~${formatSize(f.filesize_approx)}`
  return ''
}

/// 把浏览器 Cookie 读取失败的原始报错翻译成可操作的引导。
/// 其余错误原样返回，避免吞掉真实原因。
export function enhanceCookieError(msg: string): string {
  if (/Could not copy.*cookie database|database is locked|cookie.*locked/i.test(msg)) {
    return (
      msg +
      '\n\n浏览器 Cookie 数据库被占用：请完全关闭 Chrome/Edge（或结束所有相关进程）后再试，' +
      '也可以在「设置」换一个未运行的浏览器，或导出 cookies.txt 作为后续方案。'
    )
  }
  return msg
}
