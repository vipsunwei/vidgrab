<template>
  <aside class="history-panel">
    <div class="history-header">
      <span class="history-title">今日下载（{{ records.length }}）</span>
    </div>

    <div v-if="records.length" class="history-list">
      <HistoryItem
        v-for="item in records"
        :key="item.id"
        :item="item"
        :menu-open="activeMenuId === item.id"
        @open-file="emit('open-file', $event)"
        @open-folder="emit('open-folder', $event)"
        @delete-record="emit('delete-record', $event)"
        @delete-file="emit('delete-file', $event)"
        @show-details="emit('show-details', $event)"
        @redownload="emit('redownload', $event)"
        @menu-toggle="emit('menu-toggle', item.id, $event)"
      />
    </div>
    <div v-else class="history-empty">
      <span class="i-ph-clock-counter-clockwise-bold" />
      今日暂无下载记录
    </div>
  </aside>
</template>

<script setup lang="ts">
import HistoryItem from '@/components/HistoryItem.vue'
import type { DownloadRecord } from '@/types'

defineProps<{ records: DownloadRecord[]; activeMenuId: string | null }>()

const emit = defineEmits<{
  'open-file': [path: string]
  'open-folder': [path: string]
  'delete-record': [id: string]
  'delete-file': [id: string]
  'show-details': [item: DownloadRecord]
  'redownload': [item: DownloadRecord]
  'menu-toggle': [id: string, open: boolean]
}>()
</script>

<style scoped>
.history-panel {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  padding: 18px 14px;
  background: var(--v-surface);
  border-left: 1px solid var(--v-border);
  backdrop-filter: blur(12px);
  overflow-y: auto;
}

.history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
}
.history-title {
  font-size: 13px;
  font-weight: 700;
  color: var(--v-text-secondary);
  letter-spacing: 0.04em;
}

.history-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.history-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--v-text-muted);
  font-size: 13px;
}
.history-empty span {
  font-size: 32px;
  opacity: 0.45;
}
</style>
