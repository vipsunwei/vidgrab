// src/composables/useSettings.ts
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

// 全局设置（持久化到 localStorage）
// outputDir: 空字符串 = 使用系统下载目录下的 VidGrab 子目录（默认）
const OUTPUT_DIR_KEY = 'vidgrab-output-dir'
// cookieSource: 部分站点解析需浏览器活跃会话 cookie；none = 不使用
const COOKIE_SOURCE_KEY = 'vidgrab-cookie-source'

type CookieSource = 'none' | 'chrome' | 'edge'

function loadOutputDir(): string {
  try {
    return localStorage.getItem(OUTPUT_DIR_KEY) ?? ''
  } catch {
    return ''
  }
}

function loadCookieSource(): CookieSource {
  try {
    const v = localStorage.getItem(COOKIE_SOURCE_KEY)
    if (v === 'chrome' || v === 'edge' || v === 'none') return v
  } catch {
    /* 忽略读取失败 */
  }
  return 'none'
}

const outputDir = ref<string>(loadOutputDir())
const defaultOutputDir = ref<string>('')
const cookieSource = ref<CookieSource>(loadCookieSource())

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

function setCookieSource(src: CookieSource) {
  cookieSource.value = src
  try {
    localStorage.setItem(COOKIE_SOURCE_KEY, src)
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
    cookieSource,
    setCookieSource,
  }
}
