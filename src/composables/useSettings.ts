// src/composables/useSettings.ts
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

// 全局设置（持久化到 localStorage）
// outputDir: 空字符串 = 使用系统下载目录下的 VidGrab 子目录（默认）
// 注意：Cookie 由应用托管（app_data/cookies.txt，见后端 cookies.rs），
// 其启用状态与站点列表经后端命令管理，不在此处持久化。
const OUTPUT_DIR_KEY = 'vidgrab-output-dir'

function loadOutputDir(): string {
  try {
    return localStorage.getItem(OUTPUT_DIR_KEY) ?? ''
  } catch {
    return ''
  }
}

const outputDir = ref<string>(loadOutputDir())
const defaultOutputDir = ref<string>('')

const effectiveOutputDir = computed(() => {
  if (outputDir.value) return outputDir.value
  return defaultOutputDir.value || ''
})

async function loadDefaultOutputDir() {
  try {
    const path = await invoke<string>('get_default_download_dir')
    defaultOutputDir.value = path
  } catch (e) {
    console.error('获取默认下载目录失败:', e)
  }
}

function setOutputDir(path: string) {
  outputDir.value = path.trim()
  try {
    if (outputDir.value) {
      localStorage.setItem(OUTPUT_DIR_KEY, outputDir.value)
    } else {
      localStorage.removeItem(OUTPUT_DIR_KEY)
    }
  } catch {
    /* 忽略持久化失败 */
  }
}

export function useSettings() {
  return {
    outputDir,
    defaultOutputDir,
    effectiveOutputDir,
    loadDefaultOutputDir,
    setOutputDir,
  }
}
