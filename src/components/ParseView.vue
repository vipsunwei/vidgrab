<template>
  <div class="main-inner">
    <UrlBar v-model="url" :loading="loading" @parse="emit('parse')" />

    <p v-if="error" class="error-msg">
      <span class="i-ph-warning-circle-bold" />
      {{ error }}
    </p>

    <div class="workspace">
      <template v-if="metadata">
        <VideoMetaCard
          :metadata="metadata"
          @dismiss="emit('dismiss')"
        />

        <div class="tracks-block">
          <TrackPicker
            title="视频 (Video)"
            :capsules="videoCapsules"
            :selected="selVideo"
            @select="emit('select-video', $event)"
          />
          <TrackPicker
            title="音频 (Audio)"
            :capsules="audioCapsules"
            :selected="selAudio"
            @select="emit('select-audio', $event)"
          />

          <OutputOptions
            v-model:file-name="fileName"
            :output-dir="outputDir"
            @open-settings="emit('open-settings')"
          />
        </div>

        <button class="download-btn" :disabled="!canDownload" @click="emit('download')">
          <span class="i-ph-download-simple-bold" />
          开始下载并合成 MP4
        </button>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import UrlBar from '@/components/UrlBar.vue'
import VideoMetaCard from '@/components/VideoMetaCard.vue'
import TrackPicker from '@/components/TrackPicker.vue'
import OutputOptions from '@/components/OutputOptions.vue'
import type { TrackCapsule, VideoMetadata } from '@/types'

// 两个 model 透传给子组件（UrlBar / OutputOptions），填写即向父级 emit
const url = defineModel<string>('url', { required: true })
const fileName = defineModel<string>('fileName', { required: true })

defineProps<{
  loading: boolean
  error: string
  metadata: VideoMetadata | null
  videoCapsules: TrackCapsule[]
  audioCapsules: TrackCapsule[]
  selVideo: string
  selAudio: string
  outputDir: string
  canDownload: boolean
}>()

const emit = defineEmits<{
  parse: []
  dismiss: []
  'select-video': [key: string]
  'select-audio': [key: string]
  'open-settings': []
  download: []
}>()
</script>

<style scoped>
.main-inner {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px;
}

.error-msg {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin: 0;
  padding: 11px 14px;
  border-radius: var(--v-radius-sm);
  background: rgba(var(--v-red-rgb), 0.1);
  border: 1px solid rgba(var(--v-red-rgb), 0.3);
  color: var(--v-red);
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}

.workspace {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.tracks-block {
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 16px;
  border-radius: var(--v-radius-lg);
  background: var(--v-surface);
  border: 1px solid var(--v-border);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.download-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 9px;
  padding: 13px 26px;
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
.download-btn:hover:not(:disabled) {
  filter: brightness(1.08);
  box-shadow: 0 8px 26px rgba(var(--v-purple-rgb), 0.42);
  transform: translateY(-1px);
}
.download-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  box-shadow: none;
}
.download-btn span:first-child {
  font-size: 17px;
}
</style>
