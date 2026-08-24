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

      <label class="set-label">浏览器 Cookie 来源</label>
      <div class="cookie-seg">
        <button
          v-for="opt in cookieOptions"
          :key="opt.value"
          :class="{ on: cookieSource === opt.value }"
          @click="emit('set-cookie', opt.value)"
        >
          {{ opt.label }}
        </button>
      </div>
      <p class="set-hint">
        部分站点解析需要登录态 cookie。选 Chrome/Edge 前请先在对应浏览器登录并保持会话活跃。
      </p>
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
        :class="{ saved: pageSaved }"
        :disabled="pageSaved"
        @click="emit('save')"
      >
        {{ pageSaved ? '已保存 ✓' : '保存设置' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
type CookieValue = 'none' | 'chrome' | 'edge'

const dirDraft = defineModel<string>('dirDraft', { required: true })

defineProps<{
  defaultOutputDir: string
  cookieSource: CookieValue
  appVersion: string
  pageSaved: boolean
}>()

const emit = defineEmits<{
  browse: []
  'set-cookie': [v: CookieValue]
  save: []
}>()

const cookieOptions: { value: CookieValue; label: string }[] = [
  { value: 'none', label: '不使用' },
  { value: 'chrome', label: 'Chrome' },
  { value: 'edge', label: 'Edge' },
]
</script>

<style scoped>
/* 卡片玻璃质感：backdrop-filter 无原子类，保留手写 */
.settings-section {
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.set-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--v-text-secondary);
  letter-spacing: 0.03em;
}

.set-input {
  transition: border-color var(--v-transition);
}
.set-input:focus {
  border-color: rgba(var(--v-blue-rgb), 0.45);
}
.set-input::placeholder {
  color: var(--v-text-muted);
}

.set-browse {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 10px 16px;
  border-radius: var(--v-radius-sm);
  border: 1px solid var(--v-border);
  background: var(--v-surface);
  color: var(--v-text-secondary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--v-transition);
}
.set-browse:hover {
  background: rgba(var(--v-blue-rgb), 0.12);
  border-color: rgba(var(--v-blue-rgb), 0.4);
  color: var(--v-blue);
}

.set-hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.6;
  color: var(--v-text-tertiary);
}
.set-hint b {
  color: var(--v-text-secondary);
  font-weight: 600;
}

/* Cookie 来源分段控件 */
.cookie-seg {
  display: inline-flex;
  padding: 3px;
  gap: 3px;
  border-radius: var(--v-radius-sm);
  border: 1px solid var(--v-border);
  background: rgba(0, 0, 0, 0.18);
  align-self: flex-start;
}
.cookie-seg button {
  padding: 7px 18px;
  border-radius: var(--v-radius-xs);
  border: none;
  background: transparent;
  color: var(--v-text-tertiary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--v-transition);
}
.cookie-seg button:hover {
  color: var(--v-text-secondary);
}
.cookie-seg button.on {
  background: linear-gradient(90deg, var(--v-blue), var(--v-purple));
  color: var(--v-text-inverse);
  box-shadow: 0 2px 10px rgba(var(--v-purple-rgb), 0.28);
}

.set-save {
  padding: 11px 30px;
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
.set-save:hover:not(:disabled) {
  filter: brightness(1.08);
  box-shadow: 0 6px 20px rgba(var(--v-purple-rgb), 0.38);
  transform: translateY(-1px);
}
.set-save.saved {
  background: linear-gradient(90deg, var(--v-blue), var(--v-cyan));
  box-shadow: 0 4px 14px rgba(var(--v-cyan-rgb), 0.3);
  cursor: default;
}
</style>
