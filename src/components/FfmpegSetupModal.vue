<template>
  <Transition name="fade">
    <div
      v-if="show"
      class="ffmpeg-modal-overlay"
      @click.self="onOverlayClick"
    >
      <div class="ffmpeg-modal">
        <div class="ffmpeg-modal-title">
          <span class="i-ph-magic-wand-bold ffmpeg-icon" />
          首次使用，需要安装 FFmpeg
        </div>
        <p class="ffmpeg-modal-desc">
          FFmpeg 是视频处理核心，首次会自动下载安装（约 145 MB）
        </p>

        <div v-if="stage === 'idle'" class="ffmpeg-btn-wrap">
          <button class="ffmpeg-cta" @click="emit('install')">
            <span class="i-ph-download-bold" /> 开始安装
          </button>
        </div>

        <div v-else-if="stage === 'downloading'" class="ffmpeg-progress-wrap">
          <div class="ffmpeg-stage-label">正在下载 FFmpeg…</div>
          <div class="bar">
            <div class="bar-fill" :style="{ width: progress + '%' }" />
          </div>
          <div class="ffmpeg-progress-num">{{ downloaded }} / {{ total }} MB</div>
        </div>

        <div v-else-if="stage === 'extracting'" class="ffmpeg-progress-wrap">
          <div class="ffmpeg-stage-label">正在解压…</div>
          <div class="bar"><div class="bar-fill full" /></div>
        </div>

        <div v-else-if="stage === 'done'" class="ffmpeg-done">
          <span class="i-ph-check-circle-fill" /> FFmpeg 安装完成！
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import type { FfmpegStage } from '@/composables/useFfmpegSetup'

const props = defineProps<{
  show: boolean
  stage: FfmpegStage
  progress: number
  downloaded: string
  total: string
}>()

const emit = defineEmits<{ install: []; close: [] }>()

// 下载/解压过程中不允许点遮罩关闭，避免中断安装留下半截文件
function onOverlayClick() {
  if (props.stage === 'downloading' || props.stage === 'extracting') return
  emit('close')
}
</script>

<style scoped>
.ffmpeg-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
}

.ffmpeg-modal {
  width: min(420px, calc(100vw - 48px));
  padding: 26px 24px;
  border-radius: var(--v-radius-lg);
  background: var(--v-bg-elevated);
  border: 1px solid var(--v-border);
  box-shadow: var(--v-shadow);
  text-align: center;
}

.ffmpeg-modal-title {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 9px;
  font-size: 17px;
  font-weight: 700;
  color: var(--v-text);
  margin-bottom: 10px;
}
.ffmpeg-icon {
  font-size: 20px;
  color: var(--v-purple);
}

.ffmpeg-modal-desc {
  margin: 0 0 20px;
  font-size: 13px;
  line-height: 1.6;
  color: var(--v-text-tertiary);
}

.ffmpeg-cta {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 12px 28px;
  border-radius: var(--v-radius-sm);
  border: none;
  font-size: 15px;
  font-weight: 700;
  font-family: 'Segoe UI', -apple-system, sans-serif;
  cursor: pointer;
  transition: all var(--v-transition);
  background: linear-gradient(90deg, var(--v-blue), var(--v-purple));
  color: var(--v-text-inverse);
  box-shadow: 0 6px 20px rgba(var(--v-purple-rgb), 0.32);
}
.ffmpeg-cta:hover {
  filter: brightness(1.08);
  transform: translateY(-1px);
}

.ffmpeg-progress-wrap {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.ffmpeg-stage-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--v-text-secondary);
}

.bar {
  height: 8px;
  border-radius: 5px;
  background: var(--v-border);
  overflow: hidden;
}
.bar-fill {
  height: 100%;
  border-radius: 5px;
  background: linear-gradient(90deg, var(--v-blue), var(--v-purple));
  box-shadow: 0 0 12px rgba(var(--v-purple-rgb), 0.4);
  transition: width 0.3s ease;
}
.bar-fill.full {
  width: 100%;
  animation: pulse 1.2s ease-in-out infinite;
}
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.ffmpeg-progress-num {
  font-size: 12px;
  font-family: 'Courier New', monospace;
  color: var(--v-text-tertiary);
}

.ffmpeg-done {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 700;
  color: var(--v-cyan);
}
.ffmpeg-done span {
  font-size: 20px;
}

</style>
