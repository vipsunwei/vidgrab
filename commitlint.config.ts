// Commit 规范：Conventional Commits（feat/fix/docs/style/refactor/perf/test/build/ci/chore/revert）
// 格式：<type>(<scope>?): <subject>，如 `feat: 支持macOS和Linux`、`fix(parser): 空指针`
// 提交时由 .husky/commit-msg 强制校验，CI 里二次校验（防止 --no-verify 绕过）
import type { UserConfig } from '@commitlint/types'

const config: UserConfig = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    // type 与 subject 必填；其余沿用 config-conventional 默认规则
    'type-empty': [2, 'never'],
    'subject-empty': [2, 'never'],
    // 中文没有大小写概念，且允许品牌名（VidGrab）开头，关闭 subject-case 检查
    'subject-case': [0],
  },
}

export default config
