<template>
  <Transition name="fade">
    <div v-if="show" class="settings-modal-overlay" @click.self="emit('close')">
      <div class="settings-modal">
        <div class="settings-title flex items-center gap-2 text-16px font-700">
          <span class="i-ph-gear-bold text-v-blue text-18px" /> 下载设置
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
        <p class="set-hint mt-2">
          当前默认目录：<b>{{ defaultOutputDir || '系统「下载」目录下的 VidGrab 文件夹' }}</b>
          。清空并保存可恢复默认。
        </p>

        <div class="set-actions flex items-center justify-end gap-2.5 mt-5.5">
          <button
            class="set-save"
            :class="{ 'set-save-saved': justSaved }"
            :disabled="justSaved"
            @click="emit('save')"
          >
            {{ justSaved ? '已保存 ✓' : '保存' }}
          </button>
          <button class="set-cancel" @click="emit('close')">取消</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
const dirDraft = defineModel<string>('dirDraft', { required: true })

defineProps<{
  show: boolean
  defaultOutputDir: string
  justSaved: boolean
}>()

const emit = defineEmits<{
  browse: []
  save: []
  close: []
}>()
</script>

<style scoped>
/* 弹层定位与玻璃模糊：原子类不便表达，保留手写（样式分层规范的例外） */
.settings-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
}

.settings-modal {
  width: min(460px, calc(100vw - 48px));
  padding: 24px;
  border-radius: var(--v-radius-lg);
  background: var(--v-bg-elevated);
  border: 1px solid var(--v-border);
  box-shadow: var(--v-shadow);
}

.settings-title {
  margin-bottom: 18px;
}
</style>
