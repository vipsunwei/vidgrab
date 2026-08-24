// 全局轻提示：单例状态，任意模块调用 showToast 都会推到同一个浮层。
import { ref } from 'vue'

const toastMsg = ref('')
let timer: number | undefined

const DURATION = 5000

export function showToast(msg: string) {
  toastMsg.value = msg
  if (timer) clearTimeout(timer)
  timer = window.setTimeout(() => {
    toastMsg.value = ''
  }, DURATION)
}

export function useToast() {
  return { toastMsg, showToast }
}
