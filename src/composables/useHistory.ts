// 下载历史域：持久化（经 Rust 读写 app data 的 history.json）、增删、清空与菜单态。
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import { isToday } from '@/utils/format'
import { showToast } from '@/composables/useToast'
import type { DownloadRecord } from '@/types'

const HISTORY_KEY = 'vidgrab:history'
const MAX_RECORDS = 50

interface HistoryOptions {
  /// 删除记录时同步清理调用方状态（移除已完成卡片、清同会话去重键），由调用方注入，避免本域依赖任务队列
  onRemoveDoneCard?: (rec: DownloadRecord) => void
  /// 清空历史时一并收起已完成的下载卡片（文件不受影响）
  onClearDoneCards?: () => void
}

export function useHistory(options: HistoryOptions = {}) {
  const history = ref<DownloadRecord[]>([])
  /// 当前展开删除菜单的记录 id（统一管理，切换页面时自动关闭）
  const activeMenuId = ref<string | null>(null)

  const todayHistory = computed(() => history.value.filter((h) => isToday(h.createdAt)))

  function parseHistory(raw: string | null): DownloadRecord[] {
    if (!raw) return []
    try {
      const parsed = JSON.parse(raw)
      return Array.isArray(parsed) ? (parsed as DownloadRecord[]) : []
    } catch {
      return []
    }
  }

  async function load() {
    let json = ''
    try {
      json = await invoke<string>('load_history')
    } catch {
      json = ''
    }
    const parsed = parseHistory(json)
    // 一次性迁移：文件为空但 localStorage 还存有旧历史时，迁过去并清除 localStorage
    if (parsed.length === 0) {
      try {
        const legacy = parseHistory(localStorage.getItem(HISTORY_KEY))
        if (legacy.length) {
          history.value = legacy
          void save()
          localStorage.removeItem(HISTORY_KEY)
          return
        }
      } catch {
        // 迁移失败忽略
      }
    }
    history.value = parsed
  }

  async function save() {
    try {
      await invoke('save_history', { json: JSON.stringify(history.value) })
    } catch {
      // 持久化失败静默忽略
    }
  }

  function add(record: Omit<DownloadRecord, 'id' | 'createdAt'>) {
    // 按任务去重：同一下载任务只写一次；重复下载同一视频（taskId 不同）各记一条
    if (history.value.some((h) => h.taskId === record.taskId)) return
    history.value.unshift({
      ...record,
      id: `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      createdAt: Date.now(),
      size: record.size ?? null,
    })
    if (history.value.length > MAX_RECORDS) {
      history.value = history.value.slice(0, MAX_RECORDS)
    }
    void save()
  }

  function remove(id: string) {
    history.value = history.value.filter((h) => h.id !== id)
    void save()
  }

  /// 仅删除记录（保留本地文件）
  function deleteRecordOnly(id: string) {
    activeMenuId.value = null
    const rec = history.value.find((h) => h.id === id)
    remove(id)
    if (rec) options.onRemoveDoneCard?.(rec)
  }

  /// 删除记录并删除本地文件
  async function deleteRecordAndFile(id: string) {
    activeMenuId.value = null
    const rec = history.value.find((h) => h.id === id)
    if (rec?.outputPath) {
      try {
        await invoke('delete_file', { path: rec.outputPath })
      } catch (e) {
        console.error('删除本地文件失败:', e)
        showToast('本地文件删除失败（记录已删除）')
      }
    }
    remove(id)
    if (rec) options.onRemoveDoneCard?.(rec)
  }

  /// 清空模式：null 未激活；'record' 仅删记录；'files' 删记录并删本地文件
  const clearConfirm = ref<null | 'record' | 'files'>(null)
  let clearTimer: ReturnType<typeof setTimeout> | undefined

  /// 二次确认式清空：第一次点进入对应模式确认态，3 秒内再点同模式才真正执行，防止误删（尤其删文件）
  function clear(mode: 'record' | 'files') {
    if (clearConfirm.value === mode) {
      clearConfirm.value = null
      if (clearTimer) clearTimeout(clearTimer)
      if (mode === 'files') {
        void clearWithFiles()
      } else {
        history.value = []
        options.onClearDoneCards?.()
        void save()
      }
      return
    }
    clearConfirm.value = mode
    if (clearTimer) clearTimeout(clearTimer)
    clearTimer = setTimeout(() => (clearConfirm.value = null), 3000)
  }

  /// 清空记录并删除所有本地文件：逐条调后端 delete_file（文件不存在视为成功），再清记录
  async function clearWithFiles() {
    const paths = history.value.map((h) => h.outputPath).filter(Boolean)
    await Promise.all(
      paths.map((p) =>
        invoke('delete_file', { path: p }).catch((e) =>
          console.error('清空时删除本地文件失败:', e),
        ),
      ),
    )
    history.value = []
    options.onClearDoneCards?.()
    void save()
  }

  function toggleMenu(id: string, open: boolean) {
    activeMenuId.value = open ? id : null
  }

  function closeMenu() {
    activeMenuId.value = null
  }

  return {
    history,
    todayHistory,
    activeMenuId,
    clearConfirm,
    load,
    add,
    deleteRecordOnly,
    deleteRecordAndFile,
    clear,
    toggleMenu,
    closeMenu,
  }
}
