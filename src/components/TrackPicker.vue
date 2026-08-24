<template>
  <div class="track-section">
    <div class="track-section-title">{{ title }}</div>
    <div class="track-capsules">
      <button
        v-for="c in capsules"
        :key="c.key"
        :class="['capsule', { active: selected === c.key }]"
        @click="emit('select', c.key)"
      >
        <span class="capsule-check"><span class="i-ph-check-bold" /></span>
        <span class="capsule-label">{{ c.label }}</span>
        <span class="capsule-sub">{{ c.sub }}</span>
        <span class="capsule-size">{{ c.size }}</span>
      </button>
      <div v-if="!capsules.length" class="capsule-empty">无可用轨道</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { TrackCapsule } from '@/types'

defineProps<{
  title: string
  capsules: TrackCapsule[]
  selected: string
}>()
const emit = defineEmits<{ select: [key: string] }>()
</script>

<style scoped>
.track-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.track-section-title {
  font-size: 13px;
  font-weight: 700;
  color: var(--v-text-secondary);
  letter-spacing: 0.04em;
}

.track-capsules {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.capsule {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  min-width: 92px;
  padding: 10px 14px 10px 30px;
  border-radius: var(--v-radius-sm);
  border: 1px solid var(--v-border);
  background: var(--v-surface);
  color: var(--v-text-secondary);
  cursor: pointer;
  transition: all var(--v-transition);
}
.capsule:hover {
  border-color: rgba(var(--v-blue-rgb), 0.4);
  background: rgba(var(--v-blue-rgb), 0.06);
}
.capsule.active {
  border-color: var(--v-blue);
  background: rgba(var(--v-blue-rgb), 0.12);
  box-shadow: 0 0 16px rgba(var(--v-blue-rgb), 0.18);
}

/* 勾选标记：未选中时只留占位，避免选中/取消时布局跳动 */
.capsule-check {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  width: 14px;
  height: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  border: 1px solid var(--v-border);
  font-size: 9px;
  color: transparent;
  transition: all var(--v-transition);
}
.capsule.active .capsule-check {
  background: var(--v-blue);
  border-color: var(--v-blue);
  color: var(--v-text-inverse);
}

.capsule-label {
  font-size: 14px;
  font-weight: 700;
  color: var(--v-text);
  font-family: 'Courier New', monospace;
}
.capsule-sub {
  font-size: 11px;
  color: var(--v-text-tertiary);
}
.capsule-size {
  font-size: 11px;
  color: var(--v-text-muted);
  font-family: 'Courier New', monospace;
}

.capsule-empty {
  padding: 10px 14px;
  font-size: 12px;
  color: var(--v-text-muted);
}
</style>
