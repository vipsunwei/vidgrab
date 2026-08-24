<template>
  <Transition name="fade">
    <div v-if="record" class="details-modal-overlay" @click.self="emit('close')">
      <div class="details-modal">
        <div class="details-head">
          <span class="i-ph-info-bold" /> 详细信息
          <button class="details-close" title="关闭" @click="emit('close')">
            <span class="i-ph-x-bold" />
          </button>
        </div>

        <div class="details-body">
          <div class="det-thumb">
            <img v-if="record.thumbnail" :src="record.thumbnail" alt="" />
            <div v-else class="det-thumb ph">
              <span class="i-ph-video-camera-slash-bold" />
            </div>
          </div>

          <div class="det-row">
            <span class="det-key">标题</span>
            <span class="det-val">{{ record.title }}</span>
          </div>

          <div class="det-row det-row-col">
            <span class="det-key">视频链接</span>
            <div class="det-val">
              <span class="det-url-text" :title="record.url">{{ record.url }}</span>
              <div class="det-url-actions">
                <button @click="emit('copy', record.url, '链接')">复制</button>
                <button @click="emit('open-url')">浏览器打开</button>
              </div>
            </div>
          </div>

          <div class="det-row">
            <span class="det-key">平台</span>
            <span class="det-val">{{ platformText }}</span>
          </div>

          <div class="det-row">
            <span class="det-key">任务 ID</span>
            <span class="det-val det-mono">{{ record.taskId || '—' }}</span>
          </div>

          <div class="det-row">
            <span class="det-key">下载时间</span>
            <span class="det-val">{{ timeText }}</span>
          </div>

          <div class="det-row">
            <span class="det-key">文件大小</span>
            <span class="det-val">{{ formatSize(record.size) }}</span>
          </div>

          <div class="det-row det-row-col">
            <span class="det-key">保存路径</span>
            <div class="det-val">
              <span class="det-path-text" :title="record.outputPath">{{
                record.outputPath
              }}</span>
              <div class="det-path-actions">
                <button @click="emit('copy', record.outputPath, '路径')">复制</button>
                <button @click="emit('open-file', record.outputPath)">打开</button>
                <button @click="emit('open-folder', record.outputPath)">打开文件夹</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { formatSize, detectPlatform } from '@/utils/format'
import type { DownloadRecord } from '@/types'

const props = defineProps<{ record: DownloadRecord | null }>()

const emit = defineEmits<{
  close: []
  copy: [text: string, label: string]
  'open-url': []
  'open-file': [path: string]
  'open-folder': [path: string]
}>()

// 平台只依据「本条记录自己的 url」推导，绝不依赖外部传入的当前输入框内容，
// 否则地址栏输入新 URL 会改变历史记录里其他条目显示的平台
const platformText = computed(() => {
  if (!props.record) return ''
  return props.record.platform || detectPlatform(props.record.url) || '未知'
})

const timeText = computed(() =>
  props.record ? new Date(props.record.createdAt).toLocaleString('zh-CN') : '',
)
</script>

<style scoped>
.details-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
}

.details-modal {
  width: min(560px, calc(100vw - 48px));
  max-height: calc(100vh - 80px);
  display: flex;
  flex-direction: column;
  border-radius: var(--v-radius-lg);
  background: var(--v-bg-elevated);
  border: 1px solid var(--v-border);
  box-shadow: var(--v-shadow);
  overflow: hidden;
}

.details-head {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 16px 18px;
  border-bottom: 1px solid var(--v-border);
  font-size: 15px;
  font-weight: 700;
  color: var(--v-text);
}
.details-head > span:first-child {
  font-size: 18px;
  color: var(--v-blue);
}
.details-close {
  margin-left: auto;
  width: 26px;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--v-radius-xs);
  border: 1px solid transparent;
  background: transparent;
  color: var(--v-text-muted);
  font-size: 12px;
  cursor: pointer;
  transition: all var(--v-transition);
}
.details-close:hover {
  background: rgba(var(--v-red-rgb), 0.12);
  border-color: rgba(var(--v-red-rgb), 0.3);
  color: var(--v-red);
}

.details-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 18px;
  overflow-y: auto;
}

.det-thumb {
  width: 100%;
  height: 180px;
  border-radius: var(--v-radius-sm);
  overflow: hidden;
  background: var(--v-surface-active);
}
.det-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.det-thumb.ph {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--v-text-muted);
  font-size: 32px;
}

.det-row {
  display: grid;
  grid-template-columns: 84px 1fr;
  align-items: baseline;
  gap: 12px;
}
.det-row-col {
  grid-template-columns: 1fr;
  gap: 6px;
}

.det-key {
  font-size: 12px;
  font-weight: 600;
  color: var(--v-text-tertiary);
}
.det-val {
  min-width: 0;
  font-size: 13px;
  line-height: 1.6;
  color: var(--v-text);
  word-break: break-word;
}
.det-mono {
  font-family: 'Courier New', monospace;
  font-size: 12px;
}

.det-url-text,
.det-path-text {
  display: block;
  font-size: 12px;
  color: var(--v-text-secondary);
  font-family: 'Courier New', monospace;
  background: rgba(0, 0, 0, 0.2);
  padding: 8px 10px;
  border-radius: var(--v-radius-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.det-url-actions,
.det-path-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 7px;
}
.det-url-actions button,
.det-path-actions button {
  padding: 6px 14px;
  border-radius: var(--v-radius-xs);
  border: 1px solid var(--v-border);
  background: var(--v-surface);
  color: var(--v-text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--v-transition);
}
.det-url-actions button:hover,
.det-path-actions button:hover {
  background: rgba(var(--v-blue-rgb), 0.12);
  border-color: rgba(var(--v-blue-rgb), 0.35);
  color: var(--v-blue);
}

</style>
