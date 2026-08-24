<template>
  <Transition name="toast-fade">
    <div v-if="message" class="toast">{{ message }}</div>
  </Transition>
</template>

<script setup lang="ts">
defineProps<{ message: string }>()
</script>

<style scoped>
.toast {
  position: fixed;
  left: 50%;
  bottom: 32px;
  transform: translateX(-50%);
  z-index: 400;
  padding: 11px 22px;
  border-radius: var(--v-radius-full);
  background: var(--v-bg-elevated);
  border: 1px solid rgba(var(--v-blue-rgb), 0.35);
  box-shadow: var(--v-shadow);
  color: var(--v-text);
  font-size: 13px;
  font-weight: 600;
  white-space: pre-wrap;
  max-width: calc(100vw - 48px);
  pointer-events: none;
}

.toast-fade-enter-active,
.toast-fade-leave-active {
  transition:
    opacity 0.28s ease,
    transform 0.28s ease;
}
.toast-fade-enter-from,
.toast-fade-leave-to {
  opacity: 0;
  transform: translate(-50%, 12px);
}
</style>
