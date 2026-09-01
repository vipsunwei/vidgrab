<template>
  <div
    class="app-shell"
    :class="{ 'history-active': activeNav === 'history' || activeNav === 'settings' }"
  >
    <AppSidebar
      v-model="activeNav"
      :active-task-count="activeTaskCount"
      :app-version="appVersion"
    />

    <!-- 中间核心工作区 -->
    <main ref="mainScrollRef" class="main" :class="{ scrolling: bodyScrolling }">
      <ParseView
        v-if="activeNav === 'parse'"
        v-model:url="url"
        v-model:file-name="outputFileName"
        :loading="loading"
        :error="error"
        :metadata="metadata"
        :video-capsules="videoCapsules"
        :audio-capsules="audioCapsules"
        :sel-video="selVideo"
        :sel-audio="selAudio"
        :output-dir="effectiveOutputDir"
        :can-download="canDownload"
        @parse="onParse"
        @dismiss="dismiss"
        @clear-error="clearParseError"
        @select-video="selectVideo"
        @select-audio="selectAudio"
        @open-settings="openQuickSettings"
        @download="enqueueDownload"
      />

      <DownloadingView
        v-else-if="activeNav === 'downloading'"
        :display-tasks="displayTasks"
        :armed-delete-id="armedDeleteId"
        @pause="pauseTask"
        @resume="resumeTask"
        @cancel="cancelTask"
        @delete="deleteTask"
        @restart="restartTask"
        @arm-delete="armDelete"
        @open-file="openTaskFile"
        @open-folder="openTaskFolder"
        @go-parse="activeNav = 'parse'"
      />

      <HistoryView
        v-else-if="activeNav === 'history'"
        :history="history"
        :active-menu-id="activeMenuId"
        :clear-confirm="clearConfirm"
        @clear-record="clear('record')"
        @clear-files="clear('files')"
        @open-file="openHistoryFile"
        @open-folder="openHistoryFolder"
        @delete-record="deleteRecordOnly"
        @delete-file="deleteRecordAndFile"
        @show-details="openDetails"
        @redownload="redownloadFromRecord"
        @menu-toggle="onMenuToggle"
      />

      <SettingsView
        v-else-if="activeNav === 'settings'"
        v-model:dir-draft="settingsDraft"
        :default-output-dir="defaultOutputDir"
        :cookie-groups="cookieGroups"
        :app-version="appVersion"
        :page-saved="pageSaved"
        @browse="pickOutputDir"
        @add-cookie="addCookieStore"
        @remove-group="removeCookieGroup"
        @clear-cookie="clearCookieStore"
        @save="saveSettingsPage"
      />
    </main>

    <!-- 右侧今日下载 -->
    <TodayPanel
      :records="todayHistory"
      :active-menu-id="activeMenuId"
      @open-file="openHistoryFile"
      @open-folder="openHistoryFolder"
      @delete-record="deleteRecordOnly"
      @delete-file="deleteRecordAndFile"
      @show-details="openDetails"
      @redownload="redownloadFromRecord"
      @menu-toggle="onMenuToggle"
    />

    <!-- 弹窗层 -->
    <FfmpegSetupModal
      :show="showFfmpegModal"
      :stage="ffmpegStage"
      :progress="ffmpegProgress"
      :downloaded="ffmpegDownloaded"
      :total="ffmpegTotal"
      @install="installFfmpegSetup"
      @close="showFfmpegModal = false"
    />

    <QuickSettingsModal
      :show="showQuickSettings"
      v-model:dir-draft="settingsDraft"
      :default-output-dir="defaultOutputDir"
      :just-saved="justSaved"
      @browse="pickOutputDir"
      @save="saveQuickSettings"
      @close="showQuickSettings = false"
    />

    <RecordDetailsModal
      :record="detailsRecord"
      @close="closeDetails"
      @copy="copyDetailsText"
      @open-url="openDetailsUrl"
      @open-file="openHistoryFile"
      @open-folder="openHistoryFolder"
    />

    <DependencyModal
      :show="showDepDialog"
      :status="depStatus"
      :installing="depInstalling"
      :install-progress="depInstallProgress"
      :install-error="depInstallError"
      @install-ytdlp="installYtDlp"
      @install-ffmpeg="installFfmpeg"
      @close="closeDepDialog"
    />

    <ToastHost :message="toastMsg" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { openPath, revealItemInDir, openUrl } from '@tauri-apps/plugin-opener'

import AppSidebar from '@/components/AppSidebar.vue'
import ParseView from '@/components/ParseView.vue'
import DownloadingView from '@/components/DownloadingView.vue'
import HistoryView from '@/components/HistoryView.vue'
import SettingsView from '@/components/SettingsView.vue'
import TodayPanel from '@/components/TodayPanel.vue'
import FfmpegSetupModal from '@/components/FfmpegSetupModal.vue'
import QuickSettingsModal from '@/components/QuickSettingsModal.vue'
import RecordDetailsModal from '@/components/RecordDetailsModal.vue'
import DependencyModal from '@/components/DependencyModal.vue'
import ToastHost from '@/components/ToastHost.vue'

import { useSettings } from '@/composables/useSettings'
import { useDependencies } from '@/composables/useDependencies'
import { useParser } from '@/composables/useParser'
import { useDownloadQueue } from '@/composables/useDownloadQueue'
import { useHistory } from '@/composables/useHistory'
import { useFfmpegSetup } from '@/composables/useFfmpegSetup'
import { useAutoScrollbar } from '@/composables/useAutoScrollbar'
import { showToast, useToast } from '@/composables/useToast'
import { detectPlatform, parseHeight } from '@/utils/format'
import type { DownloadRecord, DownloadTask, NavKey } from '@/types'

const { toastMsg } = useToast()

// 设置
const {
  outputDir,
  defaultOutputDir,
  effectiveOutputDir,
  loadDefaultOutputDir,
  setOutputDir,
} = useSettings()

// Cookie 固定存储（app_data/cookies.txt）：按站点组管理，非空即启用
const cookieGroups = ref<string[]>([])
// 固定存储路径，启动时取一次，作为 --cookies 参数值传回后端
const cookieStorePath = ref('')
const cookieEnabled = computed(() => cookieGroups.value.length > 0)

// Cookie 存储路径：有文件就用路径，否则空串（后端据此不加 --cookies）
function resolveCookieArg(): string {
  return cookieEnabled.value ? cookieStorePath.value : ''
}

// 从后端刷新站点组列表
async function refreshCookieStore() {
  try {
    const status = await invoke<{ groups: string[] }>('cookie_store_status')
    cookieGroups.value = status.groups ?? []
  } catch {
    cookieGroups.value = []
  }
}

// 依赖安装（yt-dlp / ffmpeg）
const {
  status: depStatus,
  showDialog: showDepDialog,
  installing: depInstalling,
  installProgress: depInstallProgress,
  installError: depInstallError,
  installYtDlp,
  installFfmpeg,
  closeDialog: closeDepDialog,
} = useDependencies()

// FFmpeg 首次安装引导（与依赖弹窗里的 installFfmpeg 区分，后者走 useDependencies）
const {
  showModal: showFfmpegModal,
  stage: ffmpegStage,
  progress: ffmpegProgress,
  downloaded: ffmpegDownloaded,
  total: ffmpegTotal,
  install: installFfmpegSetup,
  detect: detectFfmpeg,
  watchProgress: watchFfmpegProgress,
} = useFfmpegSetup()

// 导航与版本
const activeNav = ref<NavKey>('parse')
const appVersion = ref('')

// 历史记录
const historyStore = useHistory({
  onRemoveDoneCard: (rec) => {
    queue.removeDoneCardForRecord(rec)
    // 记录删了就同步清同会话去重键：否则再点下载会被 finishedKeys 拦截，
    // 但记录已不存在，用户无从查看
    queue.removeFinishedKeys([rec])
  },
  onClearDoneCards: () => {
    queue.removeDoneCards()
    queue.clearFinishedKeys()
  },
})
const {
  history,
  todayHistory,
  activeMenuId,
  clearConfirm,
  deleteRecordOnly,
  deleteRecordAndFile,
  clear,
  add: addHistoryRecord,
  toggleMenu: onMenuToggle,
  closeMenu: closeHistoryMenu,
} = historyStore

// 下载队列
const queue = useDownloadQueue({
  getCookieFile: resolveCookieArg,
  getOutputDir: () => outputDir.value,
  // 跨会话去重：检查内存中的历史记录（含重启后从磁盘载入的）是否已存在同视频同画质的成品。
  // 去重键只用 url + qualityTag：文件名差异不影响「是否同一份内容」。
  // 旧记录可能缺 qualityTag（早期版本未写入），退化成只比 url，避免漏拦已有重复。
  isAlreadyDownloaded: (url, qualityTag) =>
    history.value.some((h) => {
      if (h.url !== url) return false
      if (!h.qualityTag) return true // 旧记录无画质信息：同 url 即视为已下载
      return h.qualityTag === qualityTag
    }),
  onTaskDone: (task, outputPath, title, url) => {
    const recordUrl = url || task?.url || ''
    addHistoryRecord({
      url: recordUrl,
      title: title || task?.title || recordUrl,
      platform: detectPlatform(recordUrl),
      thumbnail: task?.thumbnail ?? '',
      outputPath,
      taskId: task?.id ?? '',
      size: task ? (task.videoSize || 0) + (task.audioSize || 0) : null,
      qualityTag: task?.qualityTag ?? '',
      outputDir: task?.outputDir ?? '',
      outputName: task?.outputName ?? '',
      mode: task?.mode ?? 'merge',
      formatIds: task?.formatIds ?? [],
    })
  },
})
const {
  tasks,
  armedDeleteId,
  activeTaskCount,
  displayTasks,
  pauseTask,
  resumeTask,
  cancelTask,
  restartTask,
  deleteTask,
  armDelete,
  isDuplicate,
  enqueueFromRecord,
  persist: persistTasks,
  restore: restoreTasks,
  install: installQueue,
  uninstall: uninstallQueue,
} = queue

// 解析
const parser = useParser(resolveCookieArg)
const {
  url,
  loading,
  error,
  metadata,
  selVideo,
  selAudio,
  outputFileName,
  videoCapsules,
  audioCapsules,
  downloadMode,
  canDownload,
  selectedVideoSize,
  selectedAudioSize,
  parse,
  dismiss,
  restoreLastParse,
  selectVideo,
  selectAudio,
} = parser

// 解析动作
/// 关闭解析报错提示
function clearParseError() {
  error.value = ''
}

async function onParse() {
  // 解析新链接时清理旧的失败/取消任务，避免旧报错一直占着界面
  queue.clearFailed()
  await parse()
}

// 入队下载。enqueueLock 防止快速连点在同一视频上创建多条任务：
// 判定去重与入队之间若有任何间隙（或未来加入 await），锁能保证同一时刻只有一个入队流程走完。
let enqueueLock = false
async function enqueueDownload() {
  if (enqueueLock) {
    showToast('正在加入下载队列，请稍候')
    return
  }
  if (!canDownload.value || !url.value) return
  enqueueLock = true
  try {
  const outputName = outputFileName.value.trim()
  const formatIds =
    downloadMode.value === 'merge'
      ? [selVideo.value, selAudio.value]
      : downloadMode.value === 'audio'
        ? [selAudio.value]
        : [selVideo.value]

  // 质量标签：视频取高度（1080p），音频取码率（160k）
  const fmts = metadata.value?.formats ?? []
  const vSel = fmts.find((f) => f.format_id === selVideo.value)
  const aSel = fmts.find((f) => f.format_id === selAudio.value)
  const qualityTag =
    downloadMode.value === 'audio'
      ? aSel?.abr
        ? `${Math.round(aSel.abr)}k`
        : 'audio'
      : vSel && parseHeight(vSel.resolution) > 0
        ? `${parseHeight(vSel.resolution)}p`
        : 'video'

  // 去重：同一会话内已完成 / 队列中占用的同视频同画质，直接拦截，避免无提示重复下载
  const dupMsg = isDuplicate(url.value, qualityTag)
  if (dupMsg) {
    showToast(dupMsg)
    return
  }

  const task: DownloadTask = {
    id: crypto.randomUUID(),
    url: url.value,
    title: metadata.value?.title || url.value,
    thumbnail: metadata.value?.thumbnail || '',
    mode: downloadMode.value,
    formatIds,
    stage: 'queued',
    status: 'queued',
    videoProgress: 0,
    audioProgress: 0,
    mergeProgress: 0,
    speed: '',
    eta: '',
    outputPath: '',
    error: '',
    videoSize: selectedVideoSize.value,
    audioSize: selectedAudioSize.value,
    outputName,
    outputDir: effectiveOutputDir.value,
    qualityTag,
  }
  tasks.value.push(task)
  // 保留解析结果与选择状态：方便连续选择其他清晰度/音质再次下载，
  // 无需反复解析 URL（频繁解析会被站点限流）。解析新链接时卡片自然替换。
  error.value = ''
  queue.pumpQueue()
  // 跳转到「下载中」列表查看进度
  activeNav.value = 'downloading'
  } finally {
    enqueueLock = false
  }
}

// 历史记录「重新下载」：先清掉本地成品与碎片（文件可能损坏），再原样重建任务入队。
// 绕过 isDuplicate：用户主动重下，即便是同一画质也允许。
async function redownloadFromRecord(rec: DownloadRecord) {
  try {
    await invoke('redownload_cleanup', { outputPath: rec.outputPath })
  } catch (e) {
    console.error('清理旧文件失败（仍会继续下载）:', e)
  }
  enqueueFromRecord(rec)
  showToast('已开始重新下载')
  activeNav.value = 'downloading'
}

// 任务文件操作
async function openTaskFile(id: string) {
  const t = tasks.value.find((x) => x.id === id)
  if (!t?.outputPath) return
  await safeOpen(() => openPath(t.outputPath), '打开文件失败')
}
async function openTaskFolder(id: string) {
  const t = tasks.value.find((x) => x.id === id)
  if (!t?.outputPath) return
  await safeOpen(() => revealItemInDir(t.outputPath), '打开文件夹失败')
}

// 历史文件操作
async function openHistoryFile(path: string) {
  if (!path) return
  await safeOpen(() => openPath(path), '打开文件失败')
}
async function openHistoryFolder(path: string) {
  if (!path) return
  await safeOpen(() => revealItemInDir(path), '打开文件夹失败')
}

/// 打开本地资源失败时明确提示用户（如文件已被删除），不再静默吞掉错误
async function safeOpen(action: () => Promise<unknown>, label: string) {
  try {
    await action()
  } catch (e) {
    console.error(`${label}:`, e)
    const msg = String(e)
    // openPath 在文件不存在时通常报 NOT_FOUND / 系统错误码 2
    if (/not found|不存在|NO_SUCHFILE|ERROR_FILE_NOT_FOUND|系统找不到|2\b/i.test(msg)) {
      showToast('文件不存在，可能已被删除')
    } else {
      showToast(`${label}：${msg.slice(0, 60)}`)
    }
  }
}

// 记录详情弹窗
const detailsRecord = ref<DownloadRecord | null>(null)
function openDetails(r: DownloadRecord) {
  detailsRecord.value = r
}
function closeDetails() {
  detailsRecord.value = null
}

async function copyDetailsText(text: string, label: string) {
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    showToast(`${label}已复制`)
  } catch {
    showToast('复制失败，请手动选择')
  }
}

async function openDetailsUrl() {
  const u = detailsRecord.value?.url
  if (!u) {
    showToast('该记录未保存视频链接')
    return
  }
  try {
    await openUrl(u)
  } catch (e) {
    showToast('打开浏览器失败：' + String(e))
  }
}

// 快速设置弹窗
const showQuickSettings = ref(false)
const justSaved = ref(false)
let saveTimer: number | undefined

function openQuickSettings() {
  settingsDraft.value = effectiveOutputDir.value
  justSaved.value = false
  showQuickSettings.value = true
}

function saveQuickSettings() {
  applyOutputDirDraft()
  justSaved.value = true
  showToast('设置已保存')
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = window.setTimeout(() => {
    justSaved.value = false
    showQuickSettings.value = false
  }, 1200)
}

// 设置页
const settingsDraft = ref('')
const pageSaved = ref(false)
let pageSaveTimer: number | undefined

function saveSettingsPage() {
  applyOutputDirDraft()
  pageSaved.value = true
  showToast('设置已保存')
  if (pageSaveTimer) clearTimeout(pageSaveTimer)
  pageSaveTimer = window.setTimeout(() => {
    pageSaved.value = false
  }, 1500)
}

/// 目录草稿落盘：填了默认路径或留空都视为「使用默认」
function applyOutputDirDraft() {
  const draft = settingsDraft.value.trim()
  if (draft === defaultOutputDir.value || draft === '') {
    setOutputDir('')
  } else {
    setOutputDir(draft)
  }
}

async function pickOutputDir() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: settingsDraft.value || undefined,
      title: '选择下载保存目录',
    })
    if (typeof selected === 'string') settingsDraft.value = selected
  } catch (e) {
    console.error('选择目录失败:', e)
  }
}

/// 选择导出的 cookies.txt 加入固定存储：站点名由文件域名自动识别，同站点再添加即整站更新。
async function addCookieStore() {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Cookie 文件', extensions: ['txt'] }],
      title: '选择 cookies.txt',
    })
    if (typeof selected !== 'string') return
    const group = await invoke<string>('add_cookie_store', { source: selected })
    await refreshCookieStore()
    showToast(`已添加「${group}」的 Cookie`)
  } catch (e) {
    showToast(String(e))
  }
}

/// 删除单个站点的全部 Cookie（站点标签上的 × 按钮）
async function removeCookieGroup(group: string) {
  try {
    await invoke('remove_cookie_group', { group })
    await refreshCookieStore()
    showToast(`已删除「${group}」的 Cookie`)
  } catch (e) {
    showToast(String(e))
  }
}

/// 清空 Cookie 固定存储（清除按钮二次确认后调用）
async function clearCookieStore() {
  try {
    await invoke('clear_cookie_store')
    cookieGroups.value = []
  } catch (e) {
    showToast(String(e))
  }
}

// 滚动条自动隐藏
const mainScrollRef = ref<HTMLElement | null>(null)
const bodyScrolling = ref(false)
let bodyScrollTimer: number | null = null
function onBodyScroll() {
  bodyScrolling.value = true
  if (bodyScrollTimer) window.clearTimeout(bodyScrollTimer)
  bodyScrollTimer = window.setTimeout(() => (bodyScrolling.value = false), 800)
}
const autoScrollbar = useAutoScrollbar()

// 全局键盘：Esc 逐层关闭最上层的弹窗
function handleGlobalKeydown(e: KeyboardEvent) {
  if (e.key !== 'Escape') return
  if (detailsRecord.value) return closeDetails()
  if (showQuickSettings.value) return (showQuickSettings.value = false)
  if (showFfmpegModal.value) return (showFfmpegModal.value = false)
  if (showDepDialog.value) return (showDepDialog.value = false)
}

/// 点击空白处关闭历史记录的删除菜单
function onDocClick(e: MouseEvent) {
  if (!activeMenuId.value) return
  const t = e.target as HTMLElement | null
  if (!t) return
  if (t.closest('.hi-menu') || t.closest('button.delete')) return
  closeHistoryMenu()
}

// 进入设置页时同步目录草稿；切换页面时关闭所有历史删除菜单
watch(activeNav, (nav) => {
  if (nav === 'settings') settingsDraft.value = effectiveOutputDir.value
  closeHistoryMenu()
})

onMounted(async () => {
  restoreLastParse()
  mainScrollRef.value?.addEventListener('scroll', onBodyScroll, { passive: true })
  window.addEventListener('keydown', handleGlobalKeydown)
  document.addEventListener('click', onDocClick)
  // 关闭窗口前兜底保存一次未完成任务（正常路径下状态变更时已保存）
  window.addEventListener('beforeunload', persistTasks)
  autoScrollbar.install()

  await historyStore.load()
  await loadDefaultOutputDir()
  await restoreTasks()
  try {
    cookieStorePath.value = await invoke<string>('cookie_store_path')
  } catch {
    cookieStorePath.value = ''
  }
  await refreshCookieStore()

  appVersion.value = await invoke<string>('get_app_version')
  await installQueue()
  await watchFfmpegProgress()
  await detectFfmpeg()
})

onUnmounted(() => {
  mainScrollRef.value?.removeEventListener('scroll', onBodyScroll)
  window.removeEventListener('keydown', handleGlobalKeydown)
  window.removeEventListener('beforeunload', persistTasks)
  document.removeEventListener('click', onDocClick)
  if (bodyScrollTimer) window.clearTimeout(bodyScrollTimer)
  autoScrollbar.uninstall()
  uninstallQueue()
})
</script>

<style scoped>
.app-shell {
  height: 100%;
  display: grid;
  grid-template-columns: minmax(200px, 220px) minmax(420px, 1fr) minmax(300px, 340px);
  gap: 1px;
  position: relative;
  overflow: hidden;
  background: var(--v-bg);
  font-family: 'Segoe UI', -apple-system, BlinkMacSystemFont, sans-serif;
}
.app-shell.history-active {
  grid-template-columns: 220px minmax(480px, 1fr);
}
.app-shell.history-active .history-panel {
  display: none;
}

/* 自适应：窄窗口时先收窄右栏，极窄时隐藏右栏（历史页有完整列表） */
@media (max-width: 1240px) {
  .app-shell {
    grid-template-columns: minmax(190px, 200px) minmax(400px, 1fr) minmax(270px, 300px);
  }
  .app-shell.history-active {
    grid-template-columns: minmax(190px, 200px) minmax(440px, 1fr);
  }
}
@media (max-width: 1120px) {
  .app-shell,
  .app-shell.history-active {
    grid-template-columns: minmax(190px, 200px) minmax(400px, 1fr);
  }
  /* 双类选择器压过基础 .history-panel 规则（声明顺序无关） */
  .app-shell .history-panel {
    display: none;
  }
}

.app-shell::before {
  content: '';
  position: fixed;
  inset: -20%;
  z-index: 0;
  background:
    radial-gradient(900px 600px at 110% -10%, rgba(var(--v-purple-rgb), 0.16), transparent 60%),
    radial-gradient(800px 500px at -10% 110%, rgba(var(--v-blue-rgb), 0.12), transparent 55%);
  pointer-events: none;
}

.main {
  position: relative;
  z-index: 1;
  overflow-y: auto;
  min-width: 0;
}
</style>
