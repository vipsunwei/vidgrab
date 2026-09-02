// FFmpeg 首次安装引导：缺 ffmpeg 时弹窗，负责下载/解压进度展示与安装动作。
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { onUnmounted, ref } from 'vue'

export type FfmpegStage = 'idle' | 'downloading' | 'extracting' | 'done'

export function useFfmpegSetup() {
  const showModal = ref(false)
  const stage = ref<FfmpegStage>('idle')
  const progress = ref(0)
  const downloaded = ref('')
  const total = ref('')

  let unlisten: UnlistenFn | null = null

  // 缺 ffmpeg 时弹出引导；已安装则静默
  async function detect() {
    try {
      const sys = await invoke<{ yt_dlp: boolean; ffmpeg: boolean }>('check_system')
      if (!sys.ffmpeg) showModal.value = true
    } catch {
      // 检测失败不阻塞使用，解析/下载时会有更具体的报错
    }
  }

  async function install() {
    stage.value = 'downloading'
    try {
      await invoke<string>('install_ffmpeg')
    } catch (e) {
      alert(`FFmpeg 安装失败: ${e}`)
      stage.value = 'idle'
    }
  }

  async function watchProgress() {
    unlisten = await listen<{
      stage: string
      progress: number
      downloaded_mb: number
      total_mb: number
    }>('ffmpeg-install-progress', (ev) => {
      progress.value = ev.payload.progress
      downloaded.value = ev.payload.downloaded_mb.toFixed(0)
      total.value = ev.payload.total_mb.toFixed(0)
      if (ev.payload.stage === 'downloading') stage.value = 'downloading'
      if (ev.payload.stage === 'extracting') stage.value = 'extracting'
      if (ev.payload.stage === 'done') stage.value = 'done'
    })
  }

  onUnmounted(() => unlisten?.())

  return {
    showModal,
    stage,
    progress,
    downloaded,
    total,
    detect,
    install,
    watchProgress,
  }
}
