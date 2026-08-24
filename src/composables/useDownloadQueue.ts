// 下载任务队列域：并行调度、事件订阅、暂停/继续/取消/删除、未完成任务持久化。
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, ref, watch } from 'vue'
import { enhanceCookieError } from '@/utils/format'
import type { DownloadRecord, DownloadTask } from '@/types'

const MAX_CONCURRENT = 3
/// 未完成任务持久化上限（任务数组即创建顺序），超出丢弃最旧的，防止无限累积
const MAX_PERSISTED = 30

interface QueueOptions {
  /// 传给后端的 cookie 来源，'none' 视为不使用
  getCookieSource: () => string
  /// 输出目录兜底（任务自带快照优先）
  getOutputDir: () => string
  /// 任务完成：由调用方写入历史记录。
  /// task 可能为 undefined——前端卡片已丢失时（刷新/重启过 dev）仍要按 payload 兜底写历史
  onTaskDone: (
    task: DownloadTask | undefined,
    outputPath: string,
    title: string,
    url: string,
  ) => void
  /// 历史去重：跨会话时前端内存集合已清空，改为查历史记录中是否已存在同视频同画质的成品。
  /// 由调用方用内存中的 history 数组实现（含重启后从磁盘载入的），避免每次同步判定都走异步 invoke。
  isAlreadyDownloaded: (url: string, qualityTag: string) => boolean
}

/// 传给后端的 cookie 来源：'none' 表示不使用
function cookieArg(source: string): string | null {
  return source === 'none' ? null : source
}

export function useDownloadQueue(options: QueueOptions) {
  const tasks = ref<DownloadTask[]>([])
  /// 已完成任务的去重键集合（url|qualityTag|outputName）。
  /// 用内存集合而非仅依赖历史记录：历史是异步写入磁盘的，两次极短间隔的点击可能在
  /// 历史落盘前都查不到对方，造成重复下载。任务完成即刻入集合，可在同一会话内拦截。
  const finishedKeys = ref<Set<string>>(new Set())
  /// 失败卡片上「删除」的二次确认（3 秒内再点才生效）
  const armedDeleteId = ref<string | null>(null)
  let armedDeleteTimer: number | null = null

  // 去重键只用 url + 画质标签：同一视频同一画质的成品文件名（后端 strip_extension(outputName) + qualityTag）必相同，
  // 是否重复与用户填的文件名无关。outputName 不参与，避免默认值带 .mp4 导致前后端口径错位而漏拦。
  function dupKey(url: string, qualityTag: string): string {
    return `${url}|${qualityTag}`
  }

  let unlisteners: UnlistenFn[] = []

  const activeTaskCount = computed(
    () =>
      tasks.value.filter((t) =>
        ['downloading', 'queued', 'paused'].includes(t.status),
      ).length,
  )

  // 下载区只展示未完成任务（进行中 / 排队 / 暂停 / 报错 / 取消）。
  // 已完成任务立即消失——它的去处是右侧「历史记录 / 今日下载」，不应在此滞留。
  const displayTasks = computed<DownloadTask[]>(() =>
    tasks.value
      .filter((t) =>
        ['downloading', 'queued', 'paused', 'error', 'cancelled'].includes(
          t.status,
        ),
      )
      .reverse(),
  )

  // 任务一旦进入 done 立即从列表移除（历史记录已由 onTaskDone 写入）。
  // 不依赖「是否有其他活动任务」判断，避免最后一条完成时滞留屏幕上。
  watch(
    () => tasks.value.filter((t) => t.status === 'done').length,
    () => {
      if (tasks.value.some((t) => t.status === 'done')) {
        tasks.value = tasks.value.filter((t) => t.status !== 'done')
      }
    },
  )

  function findTask(id: string) {
    return tasks.value.find((t) => t.id === id)
  }

  function pumpQueue() {
    const active = tasks.value.filter((t) => t.status === 'downloading').length
    if (active >= MAX_CONCURRENT) return
    const next = tasks.value.find((t) => t.status === 'queued')
    if (!next) return
    next.status = 'downloading'
    next.stage = 'video'
    void startOne(next)
  }

  async function startOne(task: DownloadTask) {
    try {
      await invoke('start_download', {
        url: task.url,
        formatIds: task.formatIds,
        outputDir: task.outputDir || options.getOutputDir(),
        outputName: task.outputName || null,
        qualityTag: task.qualityTag || null,
        cookieSource: cookieArg(options.getCookieSource()),
        taskId: task.id,
      })
    } catch (e) {
      task.status = 'error'
      task.error = enhanceCookieError(String(e))
      pumpQueue()
    }
  }

  function cancelTask(id: string) {
    const task = findTask(id)
    if (!task) return
    task.status = 'cancelled'
    invoke('cancel_download', { taskId: id }).catch(() => {})
    // 取消即放弃，清掉已下载的 .part 碎片，避免目录里堆残留
    invoke('clear_task_part', { taskId: id }).catch(() => {})
    void persist()
    pumpQueue()
  }

  /// 暂停：终止进程但保留 .part 断点，等「继续下载」时自动续传
  function pauseTask(id: string) {
    const task = findTask(id)
    if (!task) return
    task.status = 'paused'
    invoke('pause_download', { taskId: id }).catch(() => {})
    // 显式保存：仅靠集合 watch 会在「暂停」这类集合不变的状态切换时漏存
    void persist()
    pumpQueue()
  }

  /// 继续：同参数重新发起下载，yt-dlp 检测同名 .part 自动断点续传
  function resumeTask(id: string) {
    const task = findTask(id)
    if (!task) return
    task.status = 'downloading'
    void persist()
    void startOne(task)
  }

  /// 取消/出错后的重新下载：先清掉该任务已下载的 .part 碎片，再用原参数从头下载，
  /// 不续传——「重新下载」语义就是换一份，续传那段可能已损坏的半成品没有意义。
  async function restartTask(id: string) {
    const task = findTask(id)
    if (!task) return
    task.status = 'downloading'
    task.error = ''
    void persist()
    // 清碎片：失败不影响主流程（最多残留旧 .part，yt-dlp 会覆盖同名输出）
    await invoke('clear_task_part', { taskId: id }).catch(() => {})
    void startOne(task)
  }

  /// 删除：进行中/暂停中会终止进程并清理 .part 残留；已完成仅移除卡片（不动成品文件）
  function deleteTask(id: string) {
    const task = findTask(id)
    if (!task) return
    if (task.status !== 'done') {
      invoke('delete_task', { taskId: id }).catch(() => {})
    }
    tasks.value = tasks.value.filter((t) => t.id !== id)
    void persist()
    pumpQueue()
  }

  /// 失败卡片上的「删除」需要二次确认，避免误删已下载的 .part 进度
  function armDelete(id: string) {
    if (armedDeleteId.value === id) {
      armedDeleteId.value = null
      if (armedDeleteTimer) window.clearTimeout(armedDeleteTimer)
      deleteTask(id)
      return
    }
    armedDeleteId.value = id
    if (armedDeleteTimer) window.clearTimeout(armedDeleteTimer)
    armedDeleteTimer = window.setTimeout(() => (armedDeleteId.value = null), 3000)
  }

  /// 解析新链接时清理旧的失败/取消任务，避免旧报错一直占着界面
  function clearFailed() {
    tasks.value = tasks.value.filter(
      (t) => t.status !== 'error' && t.status !== 'cancelled',
    )
  }

  function removeDoneCards() {
    tasks.value = tasks.value.filter((t) => t.status !== 'done')
  }

  /// 删除历史记录后同步移除对应已完成卡片，否则点「打开文件」会因文件已删而报错
  function removeDoneCardForRecord(rec: { taskId?: string; outputPath?: string }) {
    tasks.value = tasks.value.filter(
      (t) =>
        !(
          t.status === 'done' &&
          (t.id === rec.taskId || (rec.outputPath && t.outputPath === rec.outputPath))
        ),
    )
  }

  // 未完成任务持久化：应用重启后恢复（未完成的恢复为已暂停，可断点续传）
  // 只保存非 done 的任务；进度更新不触发写盘，任务集合或状态变化时保存一次。
  async function persist() {
    try {
      const active = tasks.value.filter((t) => t.status !== 'done')
      await invoke('save_tasks', { json: JSON.stringify(active.slice(-MAX_PERSISTED)) })
    } catch {
      // 持久化失败不影响主流程
    }
  }

  watch(
    () =>
      tasks.value
        .filter((t) => t.status !== 'done')
        .map((t) => `${t.id}:${t.status}`)
        .join(','),
    () => {
      void persist()
    },
  )

  /// 恢复上次退出时的未完成任务：进程已不在，统一标记为已暂停
  async function restore() {
    try {
      // 后端返回 JSON 字符串（与 load_history 同模式），必须先 parse
      const raw = await invoke<string>('load_tasks')
      const saved = JSON.parse(raw) as DownloadTask[]
      if (!Array.isArray(saved) || saved.length === 0) return
      tasks.value = saved.map((t) => ({
        ...t,
        // 暂停/下载中/排队 → 进程已不在，统一恢复为已暂停（可断点续传）；
        // 错误/已取消 → 保持原状态（保留错误信息，可「重新下载」触发断点自愈）
        status: (['paused', 'downloading', 'queued'].includes(t.status)
          ? 'paused'
          : t.status) as DownloadTask['status'],
      }))
      // 重新登记输出位置，使「删除任务」仍能清理到对应的 .part 残留
      for (const t of tasks.value) {
        void invoke('register_task_output', {
          taskId: t.id,
          outputDir: t.outputDir,
          title: t.title,
          outputName: t.outputName || null,
          qualityTag: t.qualityTag || null,
        }).catch(() => {})
      }
    } catch {
      // 恢复失败忽略，不影响主流程
    }
  }

  // 后端事件订阅
  async function install() {
    unlisteners.push(
      await listen<{
        task_id: string
        stage: string
        progress: number
        speed: string
        eta: string
        filename: string
      }>('download-progress', (ev) => {
        const t = findTask(ev.payload.task_id)
        if (!t) return
        const { stage, progress, speed, eta } = ev.payload
        t.stage = stage as DownloadTask['stage']
        t.speed = speed
        t.eta = eta
        // 断网重试提示事件（speed 位为「网络重试 x/x」）progress 为 0，不能打回进度条
        if (!speed.startsWith('网络重试')) {
          if (stage === 'video') t.videoProgress = progress
          if (stage === 'audio') t.audioProgress = progress
          if (stage === 'merge') t.mergeProgress = progress
        }
      }),
    )

    unlisteners.push(
      await listen<{
        task_id: string
        output_path: string
        title: string
        url: string
      }>('download-done', (ev) => {
        const t = findTask(ev.payload.task_id)
        // 即使前端任务卡片已丢失（刷新/重启过 dev），也按事件 payload 兜底写历史
        options.onTaskDone(
          t,
          ev.payload.output_path,
          ev.payload.title,
          ev.payload.url,
        )
        if (t) {
          t.status = 'done'
          t.stage = 'done'
          t.outputPath = ev.payload.output_path
          // 记入完成集合，供后续同视频同画质去重（见 isDuplicate）
          finishedKeys.value = new Set(finishedKeys.value).add(
            dupKey(t.url, t.qualityTag),
          )
        }
        pumpQueue()
      }),
    )

    const markStatus =
      (status: DownloadTask['status'], message?: string) =>
      (ev: { payload: { task_id: string; message?: string } }) => {
        const t = findTask(ev.payload.task_id)
        // 完成状态优先：合并秒级完成时，迟到的取消/暂停事件不允许覆盖完成卡片
        if (!t || t.status === 'done') return
        t.status = status
        if (message !== undefined) t.error = message
        pumpQueue()
      }

    unlisteners.push(
      await listen<{ task_id: string }>('download-cancelled', markStatus('cancelled')),
    )
    unlisteners.push(
      await listen<{ task_id: string }>('download-paused', markStatus('paused')),
    )
    unlisteners.push(
      await listen<{ task_id: string; message: string }>(
        'download-error',
        (ev) => markStatus('error', ev.payload.message)(ev),
      ),
    )
  }

  function uninstall() {
    unlisteners.forEach((u) => u())
    unlisteners = []
    if (armedDeleteTimer) window.clearTimeout(armedDeleteTimer)
  }

  /// 去重判定：同一会话内已完成的同视频同画质、历史记录里已存在的同键成品，
  /// 或队列中仍在占用的同键任务。返回命中原因，null 表示不重复。
  /// App.vue 在入队前调用，避免无提示重复下载；「重新下载」走专用入口，不经过此判定。
  function isDuplicate(url: string, qualityTag: string): string | null {
    const key = dupKey(url, qualityTag)
    if (finishedKeys.value.has(key)) {
      return '该视频此画质已下载过（可在历史记录中查看）'
    }
    if (options.isAlreadyDownloaded(url, qualityTag)) {
      return '该视频此画质已下载过（可在历史记录中查看）'
    }
    const inQueue = tasks.value.some(
      (t) =>
        ['downloading', 'queued', 'paused', 'error', 'cancelled'].includes(
          t.status,
        ) && dupKey(t.url, t.qualityTag) === key,
    )
    if (inQueue) return '该视频此画质的下载任务已在队列中'
    return null
  }

  /// 重新下载：从历史记录原样重建任务并立即入队，跳过 isDuplicate 去重判定
  /// （用户主动要求重下，可能是文件损坏）。调用方需先调后端 redownload_cleanup
  /// 清掉本地成品与碎片。finishedKeys 中的旧键先移除，使本次重下不受同会话已完成态拦截。
  function enqueueFromRecord(rec: DownloadRecord) {
    const key = dupKey(rec.url, rec.qualityTag)
    if (finishedKeys.value.has(key)) {
      const next = new Set(finishedKeys.value)
      next.delete(key)
      finishedKeys.value = next
    }
    const task: DownloadTask = {
      id: crypto.randomUUID(),
      url: rec.url,
      title: rec.title,
      thumbnail: rec.thumbnail,
      mode: rec.mode,
      formatIds: rec.formatIds,
      stage: 'queued',
      status: 'queued',
      videoProgress: 0,
      audioProgress: 0,
      mergeProgress: 0,
      speed: '',
      eta: '',
      outputPath: '',
      error: '',
      videoSize: null,
      audioSize: null,
      outputName: rec.outputName || '',
      outputDir: rec.outputDir || options.getOutputDir(),
      qualityTag: rec.qualityTag,
    }
    tasks.value.push(task)
    pumpQueue()
  }

  return {
    tasks,
    armedDeleteId,
    activeTaskCount,
    displayTasks,
    pumpQueue,
    cancelTask,
    pauseTask,
    resumeTask,
    restartTask,
    deleteTask,
    armDelete,
    clearFailed,
    removeDoneCards,
    removeDoneCardForRecord,
    isDuplicate,
    enqueueFromRecord,
    persist,
    restore,
    install,
    uninstall,
  }
}
