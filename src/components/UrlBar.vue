<template>
  <div class="url-card">
    <span class="i-ph-link-bold url-icon" />
    <input
      v-model="url"
      class="url-input"
      type="text"
      placeholder="粘贴视频链接（支持 B站 / YouTube）"
      :disabled="loading"
      @keydown.enter="emit('parse')"
    />
    <button
      class="parse-btn"
      :disabled="!url || loading"
      @click="emit('parse')"
    >
      <span v-if="loading" class="i-ph-spinner spinner" />
      <span v-else class="i-ph-magnifying-glass-bold" />
      {{ loading ? "解析中" : "解析" }}
    </button>
  </div>
</template>

<script setup lang="ts">
const url = defineModel<string>({ required: true });
defineProps<{ loading: boolean }>();
const emit = defineEmits<{ parse: [] }>();
</script>

<style scoped>
.url-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: var(--v-radius-lg);
  background: var(--v-surface);
  border: 1px solid var(--v-border);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  transition: border-color var(--v-transition);
}
.url-card:focus-within {
  border-color: rgba(var(--v-blue-rgb), 0.45);
}

.url-icon {
  flex-shrink: 0;
  font-size: 18px;
  color: var(--v-text-tertiary);
}

.url-input {
  flex: 1;
  min-width: 0;
  background: transparent;
  border: none;
  outline: none;
  color: var(--v-text);
  font-size: 14px;
  font-family:
    "Segoe UI",
    -apple-system,
    sans-serif;
}
.url-input::placeholder {
  color: var(--v-text-muted);
}
.url-input:disabled {
  opacity: 0.6;
}

.parse-btn {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  padding: 9px 20px;
  border-radius: var(--v-radius-sm);
  border: none;
  font-size: 14px;
  font-weight: 700;
  font-family:
    "Segoe UI",
    -apple-system,
    sans-serif;
  cursor: pointer;
  transition: all var(--v-transition);
  background: linear-gradient(90deg, var(--v-blue), var(--v-purple));
  color: var(--v-text-inverse);
  box-shadow: 0 4px 14px rgba(var(--v-purple-rgb), 0.28);
}
.parse-btn:hover:not(:disabled) {
  filter: brightness(1.08);
  box-shadow: 0 6px 20px rgba(var(--v-purple-rgb), 0.38);
  transform: translateY(-1px);
}
.parse-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  box-shadow: none;
}

.spinner {
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
