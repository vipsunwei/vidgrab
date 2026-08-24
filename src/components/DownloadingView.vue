<template>
  <div class="main-inner flex flex-col p-5">
    <div class="progress-area flex flex-col gap-3.5">
      <div class="progress-header flex items-center justify-between">
        <div class="progress-title inline-flex items-center gap-2 text-15px font-700">
          <span class="i-ph-download-simple-bold text-v-blue text-18px" />
          下载中
        </div>
      </div>

      <div v-if="displayTasks.length" class="task-list flex flex-col gap-3.5">
        <div v-for="t in displayTasks" :key="t.id" class="task-item flex flex-col">
          <DownloadPanel
            v-if="t.status !== 'error' && t.status !== 'cancelled'"
            :stage="t.stage"
            :status="t.status"
            :mode="t.mode"
            :title="t.title"
            :video-progress="t.videoProgress"
            :audio-progress="t.audioProgress"
            :merge-progress="t.mergeProgress"
            :speed="t.speed"
            :eta="t.eta"
            :output-path="t.outputPath"
            :video-size="t.videoSize"
            :audio-size="t.audioSize"
            @pause="emit('pause', t.id)"
            @resume="emit('resume', t.id)"
            @cancel="emit('cancel', t.id)"
            @delete="emit('delete', t.id)"
            @open-file="emit('open-file', t.id)"
            @open-folder="emit('open-folder', t.id)"
          />
          <div v-else class="task-failed flex items-center gap-3">
            <span class="i-ph-warning-circle-bold" />
            <div class="tf-text flex-1 min-w-0 flex flex-col gap-1">
              <div class="tf-title text-13px font-600" :title="t.title">{{ t.title }}</div>
              <div class="tf-msg">{{ t.status === 'cancelled' ? '已取消' : t.error }}</div>
            </div>
            <button class="tf-restart" @click="emit('restart', t.id)">重新下载</button>
            <button
              class="tf-delete"
              :class="{ armed: armedDeleteId === t.id }"
              @click="emit('arm-delete', t.id)"
            >
              {{ armedDeleteId === t.id ? '确认删除' : '删除' }}
            </button>
          </div>
        </div>
      </div>

      <div v-else class="dl-empty flex flex-col items-center justify-center gap-3">
        <span class="i-ph-tray-bold" />
        <p class="m-0 text-14px">暂无下载任务</p>
        <button class="dl-empty-btn" @click="emit('go-parse')">去视频解析</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import DownloadPanel from '@/components/DownloadPanel.vue'
import type { DownloadTask } from '@/types'

defineProps<{
  displayTasks: DownloadTask[]
  armedDeleteId: string | null
}>()

const emit = defineEmits<{
  pause: [id: string]
  resume: [id: string]
  cancel: [id: string]
  delete: [id: string]
  restart: [id: string]
  'arm-delete': [id: string]
  'open-file': [id: string]
  'open-folder': [id: string]
  'go-parse': []
}>()
</script>

<style scoped>
/* 失败/取消卡片：与下载面板同宽，错误信息可换行完整展示 */
.task-failed {
  padding: 14px 16px;
  border-radius: var(--v-radius-lg);
  background: rgba(var(--v-red-rgb), 0.07);
  border: 1px solid rgba(var(--v-red-rgb), 0.28);
}
.task-failed > span:first-child {
  flex-shrink: 0;
  font-size: 20px;
  color: var(--v-red);
}
.tf-title {
  color: var(--v-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tf-msg {
  font-size: 12px;
  color: var(--v-red);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}

.tf-restart,
.tf-delete {
  flex-shrink: 0;
  padding: 8px 14px;
  border-radius: var(--v-radius-sm);
  font-size: 13px;
  font-weight: 600;
  font-family: 'Segoe UI', -apple-system, sans-serif;
  cursor: pointer;
  transition: all var(--v-transition);
}
.tf-restart {
  border: 1px solid rgba(var(--v-blue-rgb), 0.4);
  background: rgba(var(--v-blue-rgb), 0.1);
  color: var(--v-blue);
}
.tf-restart:hover {
  background: rgba(var(--v-blue-rgb), 0.2);
  transform: translateY(-1px);
}
.tf-delete {
  border: 1px solid rgba(var(--v-red-rgb), 0.35);
  background: rgba(var(--v-red-rgb), 0.1);
  color: var(--v-red);
}
.tf-delete:hover {
  background: rgba(var(--v-red-rgb), 0.2);
  transform: translateY(-1px);
}
.tf-delete.armed {
  background: var(--v-red);
  border-color: var(--v-red);
  color: var(--v-text-inverse);
  font-weight: 700;
}

.dl-empty {
  padding: 64px 20px;
  border-radius: var(--v-radius-lg);
  border: 1px dashed var(--v-border);
  color: var(--v-text-muted);
}
.dl-empty > span:first-child {
  font-size: 40px;
  opacity: 0.5;
}
.dl-empty-btn {
  padding: 9px 20px;
  border-radius: var(--v-radius-sm);
  border: 1px solid rgba(var(--v-blue-rgb), 0.4);
  background: rgba(var(--v-blue-rgb), 0.1);
  color: var(--v-blue);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--v-transition);
}
.dl-empty-btn:hover {
  background: rgba(var(--v-blue-rgb), 0.2);
  transform: translateY(-1px);
}
</style>
