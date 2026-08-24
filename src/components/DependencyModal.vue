<template>
  <Transition name="fade">
    <div v-if="show" class="dep-modal-overlay">
      <div class="dep-modal">
        <div class="dep-title">
          <span class="i-ph-warning-circle-bold" /> 需要安装运行依赖
        </div>
        <p class="dep-desc">
          本程序需要以下工具才能下载视频。点击下方按钮可自动下载并安装到程序目录（无需手动配置）。
        </p>

        <DependencyRow
          name="yt-dlp"
          :ok="status.yt_dlp"
          :ok-text="'已安装'"
          :miss-text="'视频解析与下载核心'"
          :installing="installing === 'ytdlp'"
          :progress="installProgress"
          @install="emit('install-ytdlp')"
        />
        <DependencyRow
          name="FFmpeg"
          :ok="status.ffmpeg"
          :ok-text="'已安装'"
          :miss-text="'音视频合并 / 转码必需'"
          :installing="installing === 'ffmpeg'"
          :progress="installProgress"
          @install="emit('install-ffmpeg')"
        />

        <p v-if="installError" class="dep-error">安装失败：{{ installError }}</p>

        <div class="dep-actions">
          <button class="dep-close" :disabled="!allReady" @click="emit('close')">
            进入应用
          </button>
        </div>
        <p v-if="!allReady" class="dep-tip">请先安装全部依赖后再进入</p>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import DependencyRow from '@/components/DependencyRow.vue'
import type { InstallProgress, SystemStatus } from '@/composables/useDependencies'

const props = defineProps<{
  show: boolean
  status: SystemStatus
  installing: 'ytdlp' | 'ffmpeg' | null
  installProgress: InstallProgress
  installError: string
}>()

const emit = defineEmits<{ 'install-ytdlp': []; 'install-ffmpeg': []; close: [] }>()

const allReady = computed(() => props.status.yt_dlp && props.status.ffmpeg)
</script>

<style scoped>
.dep-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.72);
  backdrop-filter: blur(6px);
}

.dep-modal {
  width: min(480px, calc(100vw - 48px));
  padding: 26px 24px;
  border-radius: var(--v-radius-lg);
  background: var(--v-bg-elevated);
  border: 1px solid var(--v-border);
  box-shadow: var(--v-shadow);
}

.dep-title {
  display: flex;
  align-items: center;
  gap: 9px;
  font-size: 17px;
  font-weight: 700;
  color: var(--v-text);
  margin-bottom: 10px;
}
.dep-title > span:first-child {
  font-size: 20px;
  color: var(--v-amber, var(--v-purple));
}

.dep-desc {
  margin: 0 0 20px;
  font-size: 13px;
  line-height: 1.65;
  color: var(--v-text-tertiary);
}

.dep-error {
  margin: 12px 0 0;
  padding: 10px 12px;
  border-radius: var(--v-radius-xs);
  background: rgba(var(--v-red-rgb), 0.1);
  border: 1px solid rgba(var(--v-red-rgb), 0.3);
  color: var(--v-red);
  font-size: 12px;
  line-height: 1.6;
  word-break: break-word;
}

.dep-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
.dep-close {
  padding: 11px 28px;
  border-radius: var(--v-radius-sm);
  border: none;
  font-size: 14px;
  font-weight: 700;
  font-family: 'Segoe UI', -apple-system, sans-serif;
  cursor: pointer;
  transition: all var(--v-transition);
  background: linear-gradient(90deg, var(--v-blue), var(--v-purple));
  color: var(--v-text-inverse);
  box-shadow: 0 4px 14px rgba(var(--v-purple-rgb), 0.28);
}
.dep-close:hover:not(:disabled) {
  filter: brightness(1.08);
  transform: translateY(-1px);
}
.dep-close:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  box-shadow: none;
}

.dep-tip {
  margin: 10px 0 0;
  text-align: right;
  font-size: 12px;
  color: var(--v-text-muted);
}

</style>
