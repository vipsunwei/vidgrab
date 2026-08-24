<template>
  <aside class="sidebar flex flex-col">
    <div class="brand">
      <img class="brand-logo" :src="appIcon" alt="VidGrab" />
      <div class="brand-name">VIDGRAB</div>
      <div class="brand-line flex items-center gap-2">
        <span class="brand-sub">视频下载器</span>
        <span class="brand-ver">v{{ appVersion }}</span>
      </div>
    </div>

    <nav class="nav flex flex-col gap-1.5">
      <button
        v-for="item in navItems"
        :key="item.key"
        class="nav-item flex items-center gap-2.5"
        :class="{ active: activeNav === item.key }"
        @click="activeNav = item.key"
      >
        <span :class="item.icon" />
        <span>{{ item.label }}</span>
        <span v-if="item.key === 'downloading' && activeTaskCount > 0" class="nav-badge">
          {{ activeTaskCount }}
        </span>
      </button>
    </nav>
  </aside>
</template>

<script setup lang="ts">
import appIcon from '@/assets/app-icon.png'
import type { NavKey } from '@/types'

interface NavItem {
  key: NavKey
  label: string
  icon: string
}

const activeNav = defineModel<NavKey>({ required: true })

defineProps<{
  activeTaskCount: number
  appVersion: string
}>()

const navItems: NavItem[] = [
  { key: 'parse', label: '视频解析', icon: 'i-ph-magnifying-glass-bold' },
  { key: 'downloading', label: '下载中', icon: 'i-ph-download-simple-bold' },
  { key: 'history', label: '历史记录', icon: 'i-ph-clock-counter-clockwise-bold' },
  { key: 'settings', label: '设置', icon: 'i-ph-gear-bold' },
]
</script>

<style scoped>
.sidebar {
  position: relative;
  z-index: 1;
  padding: 22px 16px 16px;
  background: var(--v-surface);
  border-right: 1px solid var(--v-border);
  backdrop-filter: blur(12px);
}

.brand {
  display: grid;
  grid-template-columns: auto 1fr;
  grid-template-rows: auto auto;
  align-items: center;
  column-gap: 12px;
  row-gap: 3px;
  margin-bottom: 32px;
}
.brand-logo {
  grid-row: 1;
  grid-column: 1;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  object-fit: cover;
  flex-shrink: 0;
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.25);
}
.brand-name {
  grid-row: 1;
  grid-column: 2;
  font-size: 18px;
  font-weight: 900;
  letter-spacing: 0.14em;
  color: var(--v-text);
  line-height: 1.1;
}
.brand-line {
  grid-row: 2;
  grid-column: 1 / -1;
}
.brand-sub {
  font-size: 13px;
  font-weight: 500;
  color: var(--v-text-tertiary);
  letter-spacing: 0.04em;
}
.brand-ver {
  font-size: 12px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: var(--v-radius-full);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.07);
  color: var(--v-text-tertiary);
  flex-shrink: 0;
}

.nav-item {
  position: relative;
  padding: 11px 14px;
  border-radius: var(--v-radius-sm);
  border: none;
  background: transparent;
  color: var(--v-text-tertiary);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--v-transition);
  text-align: left;
}
.nav-item:hover {
  background: var(--v-surface-hover);
  color: var(--v-text-secondary);
}
.nav-item.active {
  background: linear-gradient(
    90deg,
    rgba(var(--v-blue-rgb), 0.16) 0%,
    rgba(var(--v-purple-rgb), 0.04) 100%
  );
  color: #6aa6ff;
  box-shadow: inset 0 0 0 1px rgba(var(--v-blue-rgb), 0.15);
}
.nav-item.active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 18px;
  border-radius: 0 99px 99px 0;
  background: linear-gradient(180deg, var(--v-blue), var(--v-purple));
  box-shadow: 0 0 8px rgba(var(--v-blue-rgb), 0.5);
}
.nav-item.active span:first-child {
  color: #6aa6ff;
  filter: drop-shadow(0 0 5px rgba(var(--v-blue-rgb), 0.45));
}
.nav-item span:first-child {
  font-size: 18px;
  transition: all var(--v-transition);
}

.nav-badge {
  margin-left: auto;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: var(--v-radius-full);
  background: var(--v-blue);
  color: var(--v-text-inverse);
  font-size: 11px;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
</style>
