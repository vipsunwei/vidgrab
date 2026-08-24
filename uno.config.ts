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

