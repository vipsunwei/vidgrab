<template>
  <div ref="root" class="hi-item">
    <div class="hi-row1 flex items-center gap-2.5">
      <img
        v-if="item.thumbnail && !thumbErr"
        :src="item.thumbnail"
        class="hi-thumb"
        alt=""
        @error="thumbErr = true"
      />
      <div v-else class="hi-thumb ph flex items-center justify-center">
        <span class="i-ph-video-camera-slash-bold" />
      </div>

      <div class="hi-main flex-1 min-w-0 flex flex-col items-start gap-1">
        <div class="hi-title w-full min-w-0 truncate font-600 text-12px" :title="item.title">{{ item.title }}</div>
        <div class="hi-meta flex items-center gap-1.5 text-11px">
          <span>{{ formatHistoryTime(item.createdAt) }}</span>
          <span v-if="sizeText">· {{ sizeText }}</span>
        </div>
      </div>
    </div>

    <div class="hi-actions flex items-center justify-center gap-2">
      <button title="详细信息" @click="emit('show-details', item)">
        <span class="i-ph-info-bold" />
      </button>
      <button title="打开文件" @click="emit('open-file', item.outputPath)">
        <span class="i-ph-play-bold" />
      </button>
      <button title="打开文件夹" @click="emit('open-folder', item.outputPath)">
        <span class="i-ph-folder-open-bold" />
      </button>
      <button
        class="redownload"
        title="重新下载（会先删除本地文件与碎片）"
        @click="emit('redownload', item)"
      >
        <span class="i-ph-download-simple-bold" />
      </button>
      <button
        ref="deleteBtnRef"
        class="delete"
        :title="menuOpen ? '取消' : '删除记录'"
        @click.stop="toggleConfirm"
      >
        <span v-if="menuOpen" class="i-ph-x-bold" />
        <span v-else class="i-ph-trash-bold" />
      </button>

      <div
        v-if="menuOpen"
        class="hi-menu"
        :style="{
          top: `${menuPos.top}px`,
          right: `${menuPos.right}px`,
        }"
      >
        <button class="m-record" @click.stop="emit('delete-record', item.id)">
          仅删记录
        </button>
        <button class="m-file" @click.stop="emit('delete-file', item.id)">
          删文件+记录
        </button>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { formatHistoryTime, formatSize } from "@/utils/format";
import type { DownloadRecord } from "@/types";

interface MenuPos {
  top: number;
  right: number;
}

interface HistoryItemData extends DownloadRecord {}

const props = withDefaults(
  defineProps<{ item: HistoryItemData; menuOpen?: boolean }>(),
  { menuOpen: false },
);
const emit = defineEmits<{
  "open-file": [path: string];
  "open-folder": [path: string];
  "delete-record": [id: string];
  "delete-file": [id: string];
  "show-details": [item: HistoryItemData];
  "redownload": [item: HistoryItemData];
  "menu-toggle": [open: boolean];
}>();

const thumbErr = ref(false);
const deleteBtnRef = ref<HTMLButtonElement | null>(null);
const menuPos = ref<MenuPos>({ top: 0, right: 0 });

const sizeText = computed(() => formatSize(props.item.size));

function updateMenuPos() {
  if (!deleteBtnRef.value) return;
  const rect = deleteBtnRef.value.getBoundingClientRect();
  menuPos.value = {
    top: rect.bottom + 6,
    right: window.innerWidth - rect.right,
  };
}

function toggleConfirm() {
  const willOpen = !props.menuOpen;
  if (willOpen) updateMenuPos();
  emit("menu-toggle", willOpen);
}

watch(() => props.menuOpen, (open) => {
  if (open) updateMenuPos();
});
</script>

<style scoped>
.hi-item {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 10px;
  border-radius: var(--v-radius-sm);
  background: var(--v-surface);
  border: 1px solid var(--v-border);
  transition: all var(--v-transition);
}
.hi-item:hover {
  background: var(--v-surface-hover);
  border-color: rgba(var(--v-blue-rgb), 0.25);
}

/* 缩略图：固定尺寸 + 封面裁切，无法原子化 */
.hi-thumb {
  width: 64px;
  height: 46px;
  object-fit: cover;
  border-radius: var(--v-radius-xs);
  flex-shrink: 0;
  background: var(--v-surface-active);
}
.hi-thumb.ph {
  color: var(--v-text-muted);
  font-size: 18px;
}

.hi-title {
  color: var(--v-text);
}
.hi-meta {
  color: var(--v-text-tertiary);
}

/* 操作按钮：作用于子 button 的元素选择器，无法原子化到模板 */
.hi-actions button {
  flex: 1;
  height: 30px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--v-radius-xs);
  border: 1px solid var(--v-border);
  background: var(--v-surface);
  color: var(--v-text-tertiary);
  font-size: 13px;
  cursor: pointer;
  transition: all var(--v-transition);
}
.hi-actions button:hover {
  background: rgba(var(--v-blue-rgb), 0.12);
  border-color: rgba(var(--v-blue-rgb), 0.25);
  color: var(--v-blue);
}
.hi-actions button.delete:hover {
  background: rgba(var(--v-red-rgb), 0.12);
  border-color: rgba(var(--v-red-rgb), 0.25);
  color: var(--v-red);
}
.hi-actions button.redownload:hover {
  background: rgba(var(--v-blue-rgb), 0.14);
  border-color: rgba(var(--v-blue-rgb), 0.3);
  color: var(--v-blue);
}

/* 两选项删除菜单：固定定位，避免被父容器 overflow 裁剪 */
.hi-menu {
  position: fixed;
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
}
.hi-menu button {
  width: 100%;
  white-space: nowrap;
  padding: 7px 12px;
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
.hi-menu button.m-record:hover {
  background: rgba(var(--v-blue-rgb), 0.14);
  border-color: rgba(var(--v-blue-rgb), 0.3);
  color: var(--v-blue);
}
.hi-menu button.m-file:hover {
  background: rgba(var(--v-red-rgb), 0.14);
  border-color: rgba(var(--v-red-rgb), 0.3);
  color: var(--v-red);
}
</style>
