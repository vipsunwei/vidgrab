<template>
  <div class="main-inner history-view flex flex-col gap-3.5 p-5">
    <div class="history-view-header flex items-center justify-between gap-3">
      <span class="history-view-title text-15px font-700">全部下载记录（{{ history.length }}）</span>
      <div v-if="history.length" class="clear-wrap relative">
        <button class="clear-btn" @click.stop="showClearMenu = !showClearMenu">
          清空历史记录
          <span class="caret" />
        </button>
        <div v-if="showClearMenu" class="clear-menu" @click.stop>
          <button
            class="cm-item"
            :class="{ confirm: clearConfirm === 'record' }"
            @click="onPick('record')"
          >
            {{ clearConfirm === 'record' ? '确认仅删记录?' : '仅删记录（保留文件）' }}
          </button>
          <button
            class="cm-item danger"
            :class="{ confirm: clearConfirm === 'files' }"
            @click="onPick('files')"
          >
            {{ clearConfirm === 'files' ? '确认删文件+记录?' : '删文件 + 记录' }}
          </button>
          <button class="cm-item cancel" @click="showClearMenu = false">取消</button>
        </div>
      </div>
    </div>

    <div v-if="history.length" class="history-view-list">
      <HistoryItem
        v-for="item in history"
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
    <div v-else class="history-empty">暂无下载记录</div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import HistoryItem from '@/components/HistoryItem.vue'
import type { DownloadRecord } from '@/types'

defineProps<{
  history: DownloadRecord[]
  activeMenuId: string | null
  clearConfirm: 'record' | 'files' | null
}>()

const emit = defineEmits<{
  'clear-record': []
  'clear-files': []
  'open-file': [path: string]
  'open-folder': [path: string]
  'delete-record': [id: string]
  'delete-file': [id: string]
  'show-details': [item: DownloadRecord]
  'redownload': [item: DownloadRecord]
  'menu-toggle': [id: string, open: boolean]
}>()

// 清空菜单仅本地 UI 态；确认态（clearConfirm）由父组件统一管理，3 秒超时自动取消
const showClearMenu = ref(false)

// 点击后不关闭菜单：由父组件的 clear() 二次确认机制接管——
// 第一次点进入确认态（按钮文案切换为「确认…?」），3 秒内再点同模式才真正执行。
// 真正清空后 history 为空，外层 v-if="history.length" 会让整个清空区自动消失。
function onPick(mode: 'record' | 'files') {
  if (mode === 'record') emit('clear-record')
  else emit('clear-files')
}

// 点击菜单外部关闭（菜单与按钮均已 @click.stop，不会误触）
function onDocClick(e: MouseEvent) {
  if (!showClearMenu.value) return
  const t = e.target as HTMLElement | null
  if (!t || !t.closest('.clear-wrap')) showClearMenu.value = false
}
onMounted(() => document.addEventListener('click', onDocClick))
onUnmounted(() => document.removeEventListener('click', onDocClick))
</script>

<style scoped>
.clear-btn {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 14px;
  border-radius: var(--v-radius-sm);
  border: 1px solid var(--v-border);
  background: var(--v-surface);
  color: var(--v-text-tertiary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--v-transition);
}
.clear-btn:hover {
  background: rgba(var(--v-red-rgb), 0.12);
  border-color: rgba(var(--v-red-rgb), 0.35);
  color: var(--v-red);
}
.clear-btn .caret {
  width: 0;
  height: 0;
  border-left: 4px solid transparent;
  border-right: 4px solid transparent;
  border-top: 5px solid currentColor;
}

/* 清空菜单：相对 header 定位，避免被列表区裁剪 */
.clear-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  z-index: 100;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px;
  border-radius: var(--v-radius-sm);
  background: var(--v-bg-elevated);
  border: 1px solid var(--v-border);
  box-shadow: var(--v-shadow);
  backdrop-filter: blur(12px);
  min-width: 200px;
}
.cm-item {
  width: 100%;
  white-space: nowrap;
  padding: 8px 12px;
  border-radius: var(--v-radius-xs);
  border: 1px solid transparent;
  background: var(--v-surface);
  color: var(--v-text);
  font-size: 12px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
  transition: all 0.18s;
}
.cm-item:hover {
  background: rgba(var(--v-blue-rgb), 0.14);
  border-color: rgba(var(--v-blue-rgb), 0.3);
  color: var(--v-blue);
}
.cm-item.danger:hover {
  background: rgba(var(--v-red-rgb), 0.14);
  border-color: rgba(var(--v-red-rgb), 0.3);
  color: var(--v-red);
}
.cm-item.cancel:hover {
  background: var(--v-surface-hover);
  color: var(--v-text-tertiary);
}
.cm-item.confirm {
  background: var(--v-red);
  border-color: var(--v-red);
  color: var(--v-text-inverse);
}

.history-view-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 12px;
}

.history-empty {
  padding: 64px 20px;
  text-align: center;
  color: var(--v-text-muted);
  font-size: 14px;
  border: 1px dashed var(--v-border);
  border-radius: var(--v-radius-lg);
}
</style>
