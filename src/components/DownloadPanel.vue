<template>
  <div class="dl-panel">

    <!-- 面板头部：任务标题 + 状态 + 折叠 -->
    <div class="panel-header flex items-center justify-between">
      <div class="header-right flex items-center justify-end gap-3">
        <div v-if="title" class="task-title font-600 text-13px" :title="title">{{ title }}</div>
        <div class="status-indicator flex items-center gap-1.5">
          <div class="status-dot" :class="statusClass" />
          <span>{{ statusLabel }}</span>
        </div>
        <button
          class="collapse-btn flex items-center justify-center rounded-v-xs border border-v-border"
          :class="{ collapsed }"
          :title="collapsed ? '展开' : '收起'"
          @click="collapsed = !collapsed"
        >
          <span class="chevron">❯</span>
        </button>
      </div>
    </div>

    <!-- 可折叠内容区 -->
    <div class="panel-body" :class="{ open: !collapsed }">
      <div class="panel-body-inner">
        <!-- 三阶段 -->
    <div class="stages flex flex-col">

      <div
        v-for="(stage, idx) in stageList"
        :key="stage.key"
        class="stage-wrapper flex flex-col"
      >
        <!-- 连接线（除第一个） -->
        <div v-if="idx > 0" class="connector" :class="{ active: isStageActive(stage.key) || isStageDone(stage.key) }" />

        <!-- 阶段行（暂停时不显示激活态，避免流光/扫描动画看起来仍在下载） -->
        <div class="stage-row flex items-center gap-3.5 py-1.5" :class="{ active: isStageActive(stage.key) && !isPaused, done: isStageDone(stage.key) }">
          <div
            class="stage-icon flex items-center justify-center rounded-v-xs border border-v-border"
            :class="[
              stage.key,
              { active: isStageActive(stage.key) && !isPaused, done: isStageDone(stage.key) }
            ]"
          >
            <span v-if="stage.key === 'video'" class="i-ph-film-strip-bold" />
            <span v-else-if="stage.key === 'audio'" class="i-ph-speaker-high-bold" />
            <span v-else class="i-ph-lightning-bold" />
          </div>
          <div class="stage-body">
            <div class="stage-label flex items-center justify-between">
              <span class="stage-name" :class="stage.key">{{ stage.name }}</span>
              <span class="stage-status" :class="stage.key">{{ getStageStatus(stage.key) }}</span>
            </div>
            <div class="progress-track">
              <div
                class="progress-fill"
                :style="{ width: getStageProgress(stage.key) + '%' }"
              >
                <div v-if="!isPaused" class="flow-light" />
              </div>
              <div v-if="isStageActive(stage.key) && !isPaused" class="progress-scan" />
            </div>
            <div class="stage-meta flex items-center justify-between">
              <span>{{ getStageSize(stage.key) }}</span>
              <span>{{ getStageEta(stage.key) }}</span>
            </div>
          </div>
        </div>
      </div>

    </div>

    <!-- 总进度 / 完成摘要 -->
    <div class="overall">
      <div class="overall-header flex items-baseline justify-between">
        <span class="overall-label">总体进度</span>
        <span class="overall-speed">{{ displaySpeed }}</span>
      </div>
      <div class="percent-display flex items-baseline justify-center">
        <span class="percent-num">{{ Math.floor(overallPercent) }}</span>
        <span class="percent-sym">%</span>
      </div>
      <div class="overall-bar">
        <div class="overall-fill" :style="{ width: overallPercent + '%' }">
          <div v-if="!isPaused" class="flow-light" />
        </div>
      </div>
      <div class="overall-meta flex items-center justify-between">
        <span>{{ overallSize }}</span>
        <span>{{ overallEta }}</span>
      </div>
    </div>

    <div v-if="isDone && outputPath" class="path-section">
      <div v-if="outputPath" class="success-path-row flex items-center gap-2.5">
        <code class="success-path" :title="outputPath">{{ outputPath }}</code>
        <button
          class="btn-copy"
          :class="{ copied }"
          :disabled="copied"
          @click="copyPath"
        >
          {{ copied ? '已复制' : '复制' }}
        </button>
      </div>
    </div>

    <!-- 操作按钮 -->
    <div class="actions flex justify-center gap-3">
      <template v-if="!isDone">
        <template v-if="status === 'paused'">
          <button class="btn-solid inline-flex items-center justify-center gap-2" @click="emit('resume')">
            <span class="btn-icon i-ph-play-bold" /> 继续下载
          </button>
          <button class="btn btn-cancel flex items-center justify-center gap-2" @click="emit('cancel')">
            <span class="btn-icon i-ph-x-bold" /> 取消
          </button>
          <button class="btn btn-cancel flex items-center justify-center gap-2" @click="onDeleteClick">
            <span class="btn-icon i-ph-trash-bold" /> {{ deleteArmed ? "确认删除" : "删除" }}
          </button>
        </template>
        <template v-else>
          <!-- 合并是秒级收尾动作，无法真正冻结，合并阶段不提供暂停 -->
          <button
            v-if="status === 'downloading' && stage !== 'merge'"
            class="btn-solid inline-flex items-center justify-center gap-2"
            @click="emit('pause')"
          >
            <span class="btn-icon i-ph-pause-bold" /> 暂停
          </button>
          <button class="btn btn-cancel flex items-center justify-center gap-2" @click="emit('cancel')">
            <span class="btn-icon i-ph-x-bold" /> 取消
          </button>
          <button class="btn btn-cancel flex items-center justify-center gap-2" @click="onDeleteClick">
            <span class="btn-icon i-ph-trash-bold" /> {{ deleteArmed ? "确认删除" : "删除" }}
          </button>
        </template>
      </template>
      <template v-else>
        <button class="btn-solid inline-flex items-center justify-center gap-2" @click="emit('open-folder')">
          <span class="btn-icon i-ph-folder-open-bold" /> 打开文件夹
        </button>
        <button class="btn-solid inline-flex items-center justify-center gap-2" @click="emit('open-file')">
          <span class="btn-icon i-ph-play-bold" /> 打开文件
        </button>
      </template>
    </div>

        </div>
      </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

// 全部必填：本组件仅由 DownloadingView 使用且入参齐全，
// 标可选会让「漏传」变成静默取默认值而非编译报错
interface Props {
  stage: 'video' | 'audio' | 'merge' | 'done' | 'idle' | 'queued'
  // 任务状态：paused 时显示「继续下载 / 删除」，downloading 显示「暂停 / 取消 / 删除」
  status: 'queued' | 'downloading' | 'paused' | 'done'
  // 下载模式：merge=视频+音频合并 / audio=仅音频 / video=仅视频(含音轨)
  mode: 'merge' | 'audio' | 'video'
  // 任务标题（视频名），并行下载时区分多个任务
  title: string
  videoProgress: number
  audioProgress: number
  mergeProgress: number
  speed: string
  eta: string
  outputPath: string
  // 各轨真实大小（字节），用于替换写死的 ~800 MB
  videoSize: number | null
  audioSize: number | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  cancel: []
  pause: []
  resume: []
  delete: []
  'open-file': []
  'open-folder': []
}>()

const copied = ref(false)
const collapsed = ref(false)

// 删除二次确认：第一次点击变为「确认删除」，3 秒内再点才真正删除，超时自动还原
const deleteArmed = ref(false)
const deleteTimer = ref<number | null>(null)
function onDeleteClick() {
  if (deleteArmed.value) {
    deleteArmed.value = false
    if (deleteTimer.value) window.clearTimeout(deleteTimer.value)
    emit('delete')
    return
  }
  deleteArmed.value = true
  deleteTimer.value = window.setTimeout(() => (deleteArmed.value = false), 3000)
}

async function copyPath() {
  if (!props.outputPath) return
  try {
    await navigator.clipboard.writeText(props.outputPath)
    copied.value = true
    setTimeout(() => (copied.value = false), 1500)
  } catch {
    // 复制失败静默忽略，不阻断主流程
  }
}

// 根据模式生成需要展示的阶段
const stageList = computed(() => {
  if (props.mode === 'audio') {
    return [{ key: 'audio' as const, name: '音频 AUDIO', emoji: '🎵', icon: 'i-ph-speaker-high-bold' }]
  }
  if (props.mode === 'video') {
    return [{ key: 'video' as const, name: '视频 VIDEO', emoji: '🎬', icon: 'i-ph-film-strip-bold' }]
  }
  return [
    { key: 'video' as const, name: '视频 VIDEO', emoji: '🎬', icon: 'i-ph-film-strip-bold' },
    { key: 'audio' as const, name: '音频 AUDIO', emoji: '🎵', icon: 'i-ph-speaker-high-bold' },
    { key: 'merge' as const, name: '合并 MERGE', emoji: '⚡', icon: 'i-ph-lightning-bold' },
  ]
})

function isStageActive(key: string) {
  return props.stage === key
}

function isStageDone(key: string) {
  if (props.stage === 'done') return true
  const list = stageList.value
  const idx = list.findIndex((s) => s.key === key)
  const curIdx = list.findIndex((s) => s.key === props.stage)
  return idx >= 0 && curIdx > idx
}

function getStageProgress(key: string) {
  if (props.stage === 'done') return 100
  // 单轨模式下后端统一以 video 阶段上报进度
  if (props.mode !== 'merge') {
    if (key === props.stage) return props.videoProgress
    return 0
  }
  if (key === 'video') return props.videoProgress
  if (key === 'audio') return ['audio', 'merge', 'done'].includes(props.stage) ? props.audioProgress : 0
  if (key === 'merge') return ['merge', 'done'].includes(props.stage) ? props.mergeProgress : 0
  return 0
}

function getStageStatus(key: string) {
  if (isStageDone(key)) return '100%'
  if (isStageActive(key)) return `${Math.floor(getStageProgress(key))}%`
  return '等待中'
}

function formatSize(bytes: number | null | undefined): string {
  if (!bytes || bytes <= 0) return ''
  if (bytes >= 1_000_000_000) return `~${(bytes / 1_000_000_000).toFixed(1)} GB`
  if (bytes >= 1_000_000) return `~${Math.round(bytes / 1_000_000)} MB`
  return `~${Math.round(bytes / 1_000)} KB`
}

function getStageSize(key: string) {
  if (props.mode === 'audio') {
    return props.audioSize ? formatSize(props.audioSize) : '—'
  }
  if (props.mode === 'video') {
    return props.videoSize ? formatSize(props.videoSize) : '—'
  }
  if (key === 'video') return props.videoSize ? formatSize(props.videoSize) : '—'
  if (key === 'audio') return props.audioSize ? formatSize(props.audioSize) : '—'
  if (key === 'merge') {
    const total = (props.videoSize || 0) + (props.audioSize || 0)
    return total > 0 ? formatSize(total) : '—'
  }
  return '—'
}

function getStageEta(key: string) {
  // 暂停时显示的是旧快照，会误导为仍在倒计时，统一显示占位符
  if (isPaused.value) return '—'
  if (isStageActive(key) && props.eta && props.eta !== '-') return `ETA ${props.eta}`
  return '—'
}

const overallPercent = computed(() => {
  if (props.stage === 'done') return 100
  if (props.stage === 'idle') return 0
  if (props.mode !== 'merge') return getStageProgress(stageList.value[0].key)
  return (
    getStageProgress('video') * 0.4 +
    getStageProgress('audio') * 0.4 +
    getStageProgress('merge') * 0.2
  )
})

const overallSize = computed(() => {
  if (props.stage === 'idle') return '—'
  const total = (props.videoSize || 0) + (props.audioSize || 0)
  if (props.stage === 'done') {
    return total > 0 ? `${formatSize(total)}` : '—'
  }
  return total > 0 ? formatSize(total) : '—'
})

const overallEta = computed(() => {
  if (isPaused.value) return '—'
  if (props.stage === 'idle' || props.stage === 'done') return '—'
  return props.eta && props.eta !== '-' ? `ETA ${props.eta}` : '—'
})

// 后端在合并/收尾阶段会推 speed:"-"，这里把 "-" 与空值当无速度不显示
const displaySpeed = computed(() => {
  if (isPaused.value) return ''
  const s = props.speed
  return s && s !== '-' ? s : ''
})

const isDone = computed(() => props.stage === 'done')
const isPaused = computed(() => props.status === 'paused')

const statusClass = computed(() => {
  if (props.status === 'paused') return 'idle'
  if (props.stage === 'done') return 'done'
  if (props.stage === 'idle') return 'idle'
  return 'active'
})

const statusLabel = computed(() => {
  if (props.status === 'paused') return '已暂停'
  const map: Record<string, string> = {
    idle: '等待中',
    queued: '排队中',
    video: '下载中',
    audio: '下载中',
    merge: '合并中',
    done: '100%',
  }
  return map[props.stage] || '等待中'
})
</script>

<style scoped src="./DownloadPanel.css" />

