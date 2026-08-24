<template>
  <div class="video-block">
    <button class="parse-close" title="收起" @click="emit('dismiss')">
      <span class="i-ph-x-bold" />
    </button>

    <img
      v-if="metadata.thumbnail && !thumbError"
      :src="metadata.thumbnail"
      class="video-thumb"
      alt="thumbnail"
      @error="thumbError = true"
    />
    <div v-else class="video-thumb ph">
      <span class="i-ph-video-camera-slash-bold" />
    </div>

    <div class="video-meta">
      <div class="video-title">{{ metadata.title }}</div>
      <div class="video-sub">
        <span class="sub-item">
          <span class="i-ph-user-circle-bold" />
          {{ metadata.uploader || '未知' }}
        </span>
        <span class="sub-item">
          <span class="i-ph-clock-bold" />
          {{ formatDuration(metadata.duration) }}
        </span>
        <span class="pf-tag">{{ metadata.platform || '未知平台' }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { formatDuration } from '@/utils/format'
import type { VideoMetadata } from '@/types'

defineProps<{ metadata: VideoMetadata }>()
const emit = defineEmits<{ dismiss: [] }>()

const thumbError = ref(false)
</script>

<style scoped>
.video-block {
  position: relative;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px;
  border-radius: var(--v-radius-lg);
  background: var(--v-surface);
  border: 1px solid var(--v-border);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.parse-close {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 24px;
  height: 24px;
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
.parse-close:hover {
  background: rgba(var(--v-red-rgb), 0.12);
  border-color: rgba(var(--v-red-rgb), 0.3);
  color: var(--v-red);
}

.video-thumb {
  width: 132px;
  height: 74px;
  object-fit: cover;
  border-radius: var(--v-radius-sm);
  flex-shrink: 0;
  background: var(--v-surface-active);
}
.video-thumb.ph {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--v-text-muted);
  font-size: 22px;
}

.video-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.video-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--v-text);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  padding-right: 24px;
}
.video-sub {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 12px;
  color: var(--v-text-tertiary);
}
.sub-item {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.pf-tag {
  padding: 2px 9px;
  border-radius: var(--v-radius-full);
  background: rgba(var(--v-blue-rgb), 0.12);
  border: 1px solid rgba(var(--v-blue-rgb), 0.28);
  color: var(--v-blue);
  font-weight: 600;
}
</style>
