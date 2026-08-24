<template>
  <div class="dep-row" :class="{ ok }">
    <div class="dep-row-icon">
      <span v-if="ok" class="i-ph-check-circle-bold dep-ok" />
      <span v-else class="i-ph-x-circle-bold dep-bad" />
    </div>

    <div class="dep-row-info">
      <div class="dep-row-name">{{ name }}</div>
      <div class="dep-row-sub">{{ ok ? okText : missText }}</div>
      <div v-if="installing && progress.progress < 100" class="dep-bar">
        <div class="dep-bar-fill" :style="{ width: progress.progress + '%' }" />
        <span class="dep-bar-label">
          {{ progress.progress.toFixed(0) }}% {{ progress.speed }}
        </span>
      </div>
    </div>

    <button v-if="!ok && !installing" class="dep-install-btn" @click="emit('install')">
      安装
    </button>
    <span v-if="installing" class="dep-installing">安装中…</span>
  </div>
</template>

<script setup lang="ts">
import type { InstallProgress } from '@/composables/useDependencies'

defineProps<{
  name: string
  ok: boolean
  okText: string
  missText: string
  installing: boolean
  progress: InstallProgress
}>()

const emit = defineEmits<{ install: [] }>()
</script>

<style scoped>
.dep-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border-radius: var(--v-radius-sm);
  border: 1px solid var(--v-border);
  background: var(--v-surface);
  margin-bottom: 10px;
  transition: all var(--v-transition);
}
.dep-row.ok {
  border-color: rgba(var(--v-cyan-rgb), 0.3);
  background: rgba(var(--v-cyan-rgb), 0.05);
}

.dep-row-icon {
  flex-shrink: 0;
  font-size: 20px;
}
.dep-ok {
  color: var(--v-cyan);
}
.dep-bad {
  color: var(--v-red);
}

.dep-row-info {
  flex: 1;
  min-width: 0;
}
.dep-row-name {
  font-size: 13px;
  font-weight: 700;
  color: var(--v-text);
  font-family: 'Courier New', monospace;
}
.dep-row-sub {
  font-size: 11px;
  color: var(--v-text-tertiary);
  margin-top: 2px;
}

.dep-bar {
  position: relative;
  height: 6px;
  margin-top: 8px;
  border-radius: 4px;
  background: var(--v-border);
  overflow: visible;
}
.dep-bar-fill {
  height: 100%;
  border-radius: 4px;
  background: linear-gradient(90deg, var(--v-blue), var(--v-purple));
  transition: width 0.3s ease;
}
.dep-bar-label {
  position: absolute;
  right: 0;
  top: 9px;
  font-size: 10px;
  font-family: 'Courier New', monospace;
  color: var(--v-text-tertiary);
}

.dep-install-btn {
  flex-shrink: 0;
  padding: 7px 16px;
  border-radius: var(--v-radius-xs);
  border: none;
  font-size: 13px;
  font-weight: 700;
  font-family: 'Segoe UI', -apple-system, sans-serif;
  cursor: pointer;
  transition: all var(--v-transition);
  background: linear-gradient(90deg, var(--v-blue), var(--v-purple));
  color: var(--v-text-inverse);
  box-shadow: 0 3px 10px rgba(var(--v-purple-rgb), 0.25);
}
.dep-install-btn:hover {
  filter: brightness(1.08);
  transform: translateY(-1px);
}

.dep-installing {
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--v-blue);
}
</style>
