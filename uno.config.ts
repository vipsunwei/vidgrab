import { defineConfig, presetWind3, presetIcons } from 'unocss'

export default defineConfig({
  presets: [
    presetWind3(),
    presetIcons({
      scale: 1.2,
      collections: {
        ph: () => import('@iconify-json/ph/icons.json').then((i) => i.default),
      },
    }),
  ],
  shortcuts: {
    // 跨组件通用的设置区语义类。统一一份、可被原子类覆盖，避免各组件重复手写。
    // 伪类 / hover / focus 用 UnoCSS 变体表达；显式状态类用独立 shortcut（模板 :class 绑定）。
    'set-label':
      'block mb-7px text-12px font-600 color-v-text-secondary tracking-[0.03em]',
    'set-hint': 'm-0 text-12px leading-1.6 color-v-text-tertiary',
    'set-hint b': 'color-v-text-secondary font-600',
    'set-browse':
      'shrink-0 inline-flex items-center gap-6px px-16px py-10px rounded-v-sm border border-v-border bg-v-surface color-v-text-secondary text-13px font-600 cursor-pointer transition-all hover:bg-[rgba(76,141,255,0.12)] hover:border-[rgba(76,141,255,0.4)] hover:color-v-blue',
    // 清除按钮的确认态：变红警示，提示再点一次才真删
    'set-browse-confirm':
      'border-[rgba(239,68,68,0.5)] bg-[rgba(239,68,68,0.12)] color-v-red hover:border-[rgba(239,68,68,0.7)] hover:bg-[rgba(239,68,68,0.2)]',
    'set-input':
      'transition-colors focus:border-[rgba(76,141,255,0.45)] placeholder:color-v-text-muted',
    'set-save':
      'px-30px py-11px rounded-v-sm border-none text-14px font-700 cursor-pointer transition-all bg-gradient-to-r from-v-blue to-v-purple color-v-text-inverse shadow-[0_4px_14px_rgba(168,85,247,0.28)] hover:brightness-108 hover:-translate-y-1px',
    // 保存成功态：渐变换青、去掉上浮
    'set-save-saved':
      'bg-gradient-to-r from-v-blue to-v-cyan shadow-[0_4px_14px_rgba(56,189,248,0.3)] cursor-default',
    'set-cancel':
      'px-20px py-10px rounded-v-sm border border-v-border bg-v-surface color-v-text-tertiary text-14px font-600 cursor-pointer transition-all hover:bg-v-surface-hover hover:color-v-text-secondary',
  },
  theme: {
    colors: {
      // 项目品牌色板（与 src/style.css 的 --v-* 设计 token 对应）。
      // 原文半透明白色层（surface/border/text）也在此映射，统一用 bg-v-surface / border-v-border /
      // text-v-text 等原子类；UnoCSS 会原样输出对应 CSS 值，不会引入不透明问题。
      'v-blue': '#4c8dff',
      'v-purple': '#a855f7',
      'v-cyan': '#38bdf8',
      'v-green': '#10b981',
      'v-teal': '#14b8a6',
      'v-red': '#ef4444',
      'v-orange': '#f59e0b',
      'v-surface': 'rgba(255, 255, 255, 0.035)',
      'v-surface-hover': 'rgba(255, 255, 255, 0.055)',
      'v-surface-active': 'rgba(255, 255, 255, 0.075)',
      'v-surface-solid': '#11152a',
      'v-border': 'rgba(255, 255, 255, 0.08)',
      'v-border-strong': 'rgba(255, 255, 255, 0.12)',
      'v-text': 'rgba(255, 255, 255, 0.92)',
      'v-text-secondary': 'rgba(255, 255, 255, 0.6)',
      'v-text-tertiary': 'rgba(255, 255, 255, 0.4)',
      'v-text-muted': 'rgba(255, 255, 255, 0.25)',
      'v-text-inverse': '#ffffff',
      'v-bg': '#090b12',
      'v-bg-elevated': '#0d1020',
    },
    borderRadius: {
      'v-xs': '6px',
      'v-sm': '10px',
      v: '14px',
      'v-lg': '16px',
      'v-xl': '20px',
      'v-full': '9999px',
    },
  },
})

