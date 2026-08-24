// src/composables/useDependencies.ts
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { ref, onUnmounted } from 'vue'

export interface SystemStatus {
  yt_dlp: boolean
  ffmpeg: boolean
}

export interface InstallProgress {
  progress: number
  speed: string
  downloaded_mb: number
  total_mb: number
  stage: string
}

// 全局单例状态
const status = ref<SystemStatus>({ yt_dlp: false, ffmpeg: false })
const showDialog = ref(false)
// 当前正在安装的项：'ytdlp' | 'ffmpeg' | null
const installing = ref<'ytdlp' | 'ffmpeg' | null>(null)
const installProgress = ref<InstallProgress>({
  progress: 0,
  speed: '',
  downloaded_mb: 0,
  total_mb: 0,
  stage: '',
})
const installError = ref('')
let unlisteners: UnlistenFn[] = []

async function detect() {
  try {
    const s = await invoke<SystemStatus>('check_system')
    status.value = s
    if (!s.yt_dlp || !s.ffmpeg) {
      showDialog.value = true
    }
  } catch (e) {
    console.error('依赖检测失败:', e)
  }
}

async function installYtDlp() {
  await runInstall('ytdlp', 'install_ytdlp', 'ytdlp-install-progress')
}

async function installFfmpeg() {
  await runInstall('ffmpeg', 'install_ffmpeg', 'ffmpeg-install-progress')
}

async function runInstall(
  kind: 'ytdlp' | 'ffmpeg',
  cmd: string,
  eventName: string,
) {
  installing.value = kind
  installError.value = ''
  installProgress.value = {
    progress: 0,
    speed: '',
    downloaded_mb: 0,
    total_mb: 0,
    stage: 'downloading',
  }

  const un = await listen<InstallProgress>(eventName, (e) => {
    installProgress.value = e.payload
  })
  unlisteners.push(un)

  try {
    await invoke(cmd)
    // 重新检测
    const s = await invoke<SystemStatus>('check_system')
    status.value = s
    if (status.value.yt_dlp && status.value.ffmpeg) {
      showDialog.value = false
    }
  } catch (e) {
    installError.value = String(e)
  } finally {
    installing.value = null
  }
}

function closeDialog() {
  // 仅在两项都满足时才允许关闭（否则会影响功能）
  if (status.value.yt_dlp && status.value.ffmpeg) {
    showDialog.value = false
  }
}

export function useDependencies() {
  // 注册全局监听（仅一次）
  if (unlisteners.length === 0) {
    detect()
  }

  onUnmounted(() => {
    unlisteners.forEach((u) => u())
    unlisteners = []
  })

  return {
    status,
    showDialog,
    installing,
    installProgress,
    installError,
    detect,
    installYtDlp,
    installFfmpeg,
    closeDialog,
  }
}
