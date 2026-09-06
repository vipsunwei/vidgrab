// 一键发版：升版本号 → 生成 CHANGELOG → 提交推送 → 打 tag 推送（tag 推送触发 Release 工作流）
// 用法：
//   pnpm release                # 升 patch（如 0.0.2 -> 0.0.3）
//   pnpm release patch          # 同上
//   pnpm release minor          # 升 minor（0.0.2 -> 0.1.0）
//   pnpm release major          # 升 major（0.0.2 -> 1.0.0）
//   pnpm release 0.1.0          # 指定版本号
//   pnpm release --dry-run      # 预演：不写文件、不做任何 git 操作，只打印将执行的动作
// 前置要求：工作区干净（发版提交只含版本号变更，不卷其他改动）。
import { readFileSync, writeFileSync } from 'node:fs'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const root = path.resolve(fileURLToPath(new URL('..', import.meta.url)))
const dry = process.argv.includes('--dry-run')
const git = (cmd) => execSync(`git ${cmd}`, { cwd: root, encoding: 'utf8' }).trim()
const run = (cmd, cwd = root) => {
  if (dry) {
    console.log(`[dry-run] 将执行: ${cmd}`)
    return
  }
  execSync(cmd, { cwd, stdio: 'inherit' })
}
const write = (rel, content) => {
  if (dry) {
    console.log(`[dry-run] 将更新: ${rel}`)
    return
  }
  writeFileSync(path.join(root, rel), content)
}
const die = (msg) => {
  console.error(`[release] ${msg}`)
  process.exit(1)
}

// 1. 工作区必须干净（预演模式放行，但提示）
if (git('status --porcelain')) {
  if (dry) {
    console.log('[dry-run] 注意：工作区有未提交改动，正式发版时要求先提交干净')
  } else {
    console.error('[release] 工作区有未提交改动，先提交或 stash 再发版：')
    console.error(git('status --short'))
    process.exit(1)
  }
}

// 2. 目标版本：参数指定，否则当前版本 patch +1
const CONF = 'src-tauri/tauri.conf.json'
const TOML = 'src-tauri/Cargo.toml'
const cur = JSON.parse(readFileSync(path.join(root, CONF), 'utf8')).version
const posArgs = process.argv.slice(2).filter((a) => a !== '--dry-run')
let ver = posArgs[0] ?? ''
if (!ver || ver === 'patch' || ver === 'minor' || ver === 'major') {
  const [a, b, c] = cur.split('.').map(Number)
  if (ver === 'major') ver = `${a + 1}.0.0`
  else if (ver === 'minor') ver = `${a}.${b + 1}.0`
  else ver = `${a}.${b}.${c + 1}`
}
if (!/^\d+\.\d+\.\d+$/.test(ver)) {
  die(`版本号应为 x.y.z，收到："${ver}"`)
}
if (ver === cur) {
  die(`版本号没变（仍是 ${cur}），至少要升一位`)
}
if (git(`tag -l v${ver}`)) {
  die(`tag v${ver} 已存在，换个版本号或先删旧 tag`)
}

// 3. 改版本号：tauri.conf.json（CI 校验的权威来源）、Cargo.toml、package.json
const conf = readFileSync(path.join(root, CONF), 'utf8')
write(CONF, conf.replace(`"version": "${cur}"`, `"version": "${ver}"`))
const toml = readFileSync(path.join(root, TOML), 'utf8')
write(TOML, toml.replace(/^version = ".*"/m, `version = "${ver}"`))
const pkg = readFileSync(path.join(root, 'package.json'), 'utf8')
write('package.json', pkg.replace(`"version": "${cur}"`, `"version": "${ver}"`))

// 4. 把未发布提交记为 v${ver} 写入 CHANGELOG.md，随版本号一起提交；
//    tag 打在这个提交上，CI 的 git-cliff --latest 生成出与之同源的 Release 正文
try {
  execSync('git-cliff --version', { stdio: 'pipe' })
} catch {
  die('未安装 git-cliff，先安装：cargo install git-cliff')
}
if (dry) {
  // 真实执行：git-cliff 只读、输出到 stdout，让用户预览本版正文
  execSync(`git-cliff --unreleased --strip all --tag v${ver}`, { cwd: root, stdio: 'inherit' })
} else {
  run(`git-cliff -o CHANGELOG.md --unreleased --tag v${ver}`)
}

// 5. 同步 Cargo.lock 的自身版本
run('cargo update -p vidgrab --offline', path.join(root, 'src-tauri'))

// 6. 提交推送 + 打 tag 推送（tag 指向含 changelog 的提交）
run(`git add ${CONF} ${TOML} src-tauri/Cargo.lock package.json CHANGELOG.md`)
run(`git commit -m "chore: release v${ver}"`)
run('git push')
run(`git tag v${ver}`)
run(`git push origin v${ver}`)

if (dry) {
  console.log('\n[dry-run] 预演结束：未修改任何文件、未执行任何 git 操作。')
} else {
  console.log(`\n[release] v${ver} 已推送。`)
  console.log('[release] 去 GitHub Actions 看三平台构建，完成后到 Releases 页把草稿 Publish。')
}
