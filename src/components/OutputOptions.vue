<template>
  <div class="output-grid">
    <div class="output-field">
      <label>输出文件名</label>
      <div class="output-input-wrap">
        <input v-model="fileName" type="text" placeholder="留空则使用视频标题" />
      </div>
    </div>
    <div class="output-field">
      <label>保存位置</label>
      <div class="output-input-wrap">
        <input :value="outputDir" type="text" readonly />
        <button title="下载设置" @click="emit('open-settings')">
          <span class="i-ph-gear-bold" />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const fileName = defineModel<string>('fileName', { required: true })
defineProps<{ outputDir: string }>()
const emit = defineEmits<{ 'open-settings': [] }>()
</script>

<style scoped>
.output-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}

.output-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}
.output-field label {
  font-size: 12px;
  font-weight: 600;
  color: var(--v-text-secondary);
  letter-spacing: 0.03em;
}

.output-input-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  border-radius: var(--v-radius-sm);
  border: 1px solid var(--v-border);
  background: rgba(0, 0, 0, 0.18);
  transition: border-color var(--v-transition);
}
.output-input-wrap:focus-within {
  border-color: rgba(var(--v-blue-rgb), 0.45);
}

.output-input-wrap input {
  flex: 1;
  min-width: 0;
  padding: 9px 0;
  background: transparent;
  border: none;
  outline: none;
  color: var(--v-text);
  font-size: 13px;
  font-family: 'Segoe UI', -apple-system, sans-serif;
}
.output-input-wrap input::placeholder {
  color: var(--v-text-muted);
}
.output-input-wrap input[readonly] {
  color: var(--v-text-tertiary);
}

.output-input-wrap button {
  flex-shrink: 0;
  width: 26px;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--v-radius-xs);
  border: 1px solid transparent;
  background: transparent;
  color: var(--v-text-tertiary);
  font-size: 14px;
  cursor: pointer;
  transition: all var(--v-transition);
}
.output-input-wrap button:hover {
  background: rgba(var(--v-blue-rgb), 0.12);
  border-color: rgba(var(--v-blue-rgb), 0.3);
  color: var(--v-blue);
}

@media (max-width: 720px) {
  .output-grid {
    grid-template-columns: 1fr;
  }
}
</style>
