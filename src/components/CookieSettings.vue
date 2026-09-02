<!-- Cookie 设置区块：存储由应用管理（app_data/cookies.txt），
     用户只按站点添加/删除，不关心文件路径与具体域名。 -->
<template>
  <div class="flex flex-col gap-2.5">
    <p class="set-hint">
      部分站点解析需要登录态。用浏览器扩展（如 Get cookies.txt LOCALLY）在目标站点登录后
      导出 cookies.txt，添加后该站点即可解析下载。同一站点再次添加即整站更新。
    </p>
    <div class="flex items-center gap-2.5 flex-wrap">
      <button class="set-browse" type="button" @click="emit('add-cookie')">添加 Cookie 文件</button>
      <button
        v-if="cookieGroups.length"
        class="set-browse"
        :class="{ 'set-browse-confirm': clearArmed }"
        type="button"
        @click="onClearClick"
      >
        {{ clearArmed ? '确认删除' : '清除全部' }}
      </button>
    </div>
    <p v-if="!cookieGroups.length" class="set-hint">尚未添加任何站点 Cookie。</p>
    <div v-else class="group-list flex flex-wrap gap-2">
      <span v-for="g in cookieGroups" :key="g" class="group-chip">
        {{ g }}
        <button
          class="group-remove"
          type="button"
          :title="`删除 ${g} 的 Cookie`"
          @click="emit('remove-group', g)"
        >
          <span class="i-ph-x-bold" />
        </button>
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onUnmounted, ref } from 'vue'

defineProps<{
  cookieGroups: string[]
}>()

const emit = defineEmits<{
  'add-cookie': []
  'remove-group': [g: string]
  'clear-cookie': []
}>()

// 清空的二次确认：第一次点进入确认态，3 秒内再点才真正清除
const clearArmed = ref(false)
let clearTimer: ReturnType<typeof setTimeout> | undefined

function onClearClick() {
  if (clearArmed.value) {
    clearArmed.value = false
    if (clearTimer) clearTimeout(clearTimer)
    emit('clear-cookie')
    return
  }
  clearArmed.value = true
  if (clearTimer) clearTimeout(clearTimer)
  clearTimer = setTimeout(() => (clearArmed.value = false), 3000)
}

onUnmounted(() => {
  if (clearTimer) clearTimeout(clearTimer)
})
</script>

<style scoped>
/* 站点标签样式为组件独有（受 scoped 隔离），不共享，故保留在本地 */
/* 其余 set-* 已收敛到 uno.config.ts 的 shortcuts（可被原子类覆盖） */

.group-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 5px 6px 5px 11px;
  border-radius: var(--v-radius-full);
  border: 1px solid var(--v-border);
  background: var(--v-surface);
  color: var(--v-text-secondary);
  font-size: 12.5px;
  font-weight: 600;
}
.group-remove {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  border: none;
  border-radius: var(--v-radius-full);
  background: transparent;
  color: var(--v-text-muted);
  font-size: 11px;
  cursor: pointer;
  transition: all var(--v-transition);
}
.group-remove:hover {
  background: rgba(var(--v-red-rgb), 0.15);
  color: var(--v-red);
}
</style>
