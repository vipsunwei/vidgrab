// 滚动条自动隐藏：容器滚动时加 .scrolling，静止后移除，使滚动条淡出。
// 用捕获阶段监听 window，覆盖所有可滚动后代容器，无需逐个绑定。
const SCROLLING_CLASS = 'scrolling'
const IDLE_MS = 700

export function useAutoScrollbar() {
  const timers = new WeakMap<HTMLElement, number>()

  const onScroll = (e: Event) => {
    const el = e.target as HTMLElement | null
    if (!el || el.nodeType !== 1) return
    el.classList.add(SCROLLING_CLASS)
    const prev = timers.get(el)
    if (prev) window.clearTimeout(prev)
    timers.set(
      el,
      window.setTimeout(() => el.classList.remove(SCROLLING_CLASS), IDLE_MS),
    )
  }

  return {
    install: () => window.addEventListener('scroll', onScroll, true),
    uninstall: () => window.removeEventListener('scroll', onScroll, true),
  }
}
