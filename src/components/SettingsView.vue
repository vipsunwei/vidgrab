<template>
  <div class="main-inner settings-view flex flex-col gap-4.5 p-5">
    <div class="settings-page-header flex items-center gap-2.5 text-17px font-700">
      <span class="i-ph-gear-bold text-v-blue text-20px" />
      <span>设置</span>
    </div>

    <div class="settings-section flex flex-col gap-2.5 rounded-v-lg border border-v-border bg-v-surface p-4.5">
      <div class="settings-section-title flex items-center gap-2 text-14px font-700">
        <span class="i-ph-download-simple-bold text-v-blue text-17px" />
        下载设置
      </div>

      <label class="set-label">保存目录</label>
      <div class="set-input-row flex items-center gap-2">
        <input
          v-model="dirDraft"
          class="set-input flex-1 min-w-0 rounded-v-sm border border-v-border bg-[rgba(0,0,0,0.18)] px-3 py-2.5 text-13px text-v-text outline-none"
          type="text"
          placeholder="留空 = 系统下载目录 / VidGrab"
        />
        <button class="set-browse" @click="emit('browse')">
          <span class="i-ph-folder-open-bold" /> 浏览
        </button>
      </div>
      <p class="set-hint">
        当前默认目录：<b>{{ defaultOutputDir || '系统「下载」目录下的 VidGrab 文件夹' }}</b>
        。清空并保存可恢复默认。
      </p>

      <CookieSettings
        :cookie-groups="cookieGroups"
        @add-cookie="emit('add-cookie')"
        @remove-group="(g) => emit('remove-group', g)"
        @clear-cookie="emit('clear-cookie')"
      />
    </div>

    <div class="settings-section flex flex-col gap-2.5 rounded-v-lg border border-v-border bg-v-surface p-4.5">
      <div class="settings-section-title flex items-center gap-2 text-14px font-700">
        <span class="i-ph-info-bold text-v-blue text-17px" />
        关于
      </div>
      <p class="set-hint">VidGrab v{{ appVersion }} — 全网视频下载器。更多功能陆续上线。</p>
    </div>

    <div class="settings-page-actions flex justify-end">
      <button
        class="set-save"
        :class="{ 'set-save-saved': pageSaved }"
        :disabled="pageSaved"
        @click="emit('save')"
      >
        {{ pageSaved ? '已保存 ✓' : '保存设置' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import CookieSettings from '@/components/CookieSettings.vue'

const dirDraft = defineModel<string>('dirDraft', { required: true })

defineProps<{
  defaultOutputDir: string
  cookieGroups: string[]
  appVersion: string
  pageSaved: boolean
}>()

const emit = defineEmits<{
  browse: []
  'add-cookie': []
  'remove-group': [g: string]
  'clear-cookie': []
  save: []
}>()
</script>

<style scoped>
/* 卡片玻璃质感：backdrop-filter 无原子类，保留手写（样式分层规范的例外） */
.settings-section {
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}
</style>
