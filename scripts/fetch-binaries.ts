// 构建期拉取当前平台的 yt-dlp 与 ffmpeg 到 src-tauri/bin/，供 tauri 打包为应用资源。
// - 已存在则跳过（离线/重复构建不重新下载）；设 FORCE_BINARIES=1 强制重新拉取
// - Windows 的 yt-dlp.exe 同 unix 版一样构建时拉取（仓库不存二进制）
// - 体积大的二进制（yt-dlp unix 版、ffmpeg 全平台）不入库，各构建机首次构建时自动下载
//   （用 node 原生 TS 类型剥离直接运行：node scripts/fetch-binaries.ts，需 Node ≥22.18）
//
// 两个二进制都只取 ffmpeg 单体、不打 ffprobe（全项目只用 ffmpeg 做分轨合并）。
// 来源均在 GitHub：CI 同机房直连 + 本脚本重试/超时 + 工作流 actions/cache 三重保障。
import { existsSync, mkdirSync, readFileSync } from 'node:fs'
import { chmod, readFile, rename, rm, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { createHash } from 'node:crypto'
import zlib from 'node:zlib'
import { spawnSync } from 'node:child_process'

// 国内开发机访问 GitHub 不稳定：本机拉取时请用 Node ≥22.15 的代理开关走代理，
// 例如：NODE_USE_ENV_PROXY=1 HTTPS_PROXY=http://127.0.0.1:7897 pnpm run fetch:binaries
// CI 同机房直连，不设置任何代理环境变量，行为不变。

const REPO_ROOT: string = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '..',
)
const BIN_DIR: string = path.join(REPO_ROOT, 'src-tauri', 'bin')

/** 版本清单：记录 bin/ 下各二进制对应的固定版本（gitignore，不入库）。
 *  「已存在则跳过」据此判断现有文件是不是当前期望版本，防止旧版本被静默跳过。 */
const LOCK_PATH = path.join(BIN_DIR, 'binaries.lock.json')

async function readLock(): Promise<Record<string, string>> {
  try {
    return JSON.parse(await readFile(LOCK_PATH, 'utf8'))
  } catch {
    return {}
  }
}

async function updateLock(key: string, version: string): Promise<void> {
  const lock = await readLock()
  lock[key] = version
  await writeFile(LOCK_PATH, JSON.stringify(lock, null, 2) + '\n')
}

/** SHA256 清单：记录 bin/ 下各二进制对应固定版本的哈希，供 Rust 侧运行时校验
 *  （构建期算好固化，安装器下载同版本后比对，防传输层篡改/损坏）。 */
const SHA_PATH = path.join(BIN_DIR, 'binaries.sha256.json')

interface ShaEntry {
  version: string
  [k: string]: string
}
interface ShaMap {
  'yt-dlp': ShaEntry
  ffmpeg: ShaEntry
  python: ShaEntry
}

async function readSha(): Promise<ShaMap> {
  try {
    const v = JSON.parse(await readFile(SHA_PATH, 'utf8'))
    return {
      'yt-dlp': v['yt-dlp'] ?? { version: '' },
      ffmpeg: v.ffmpeg ?? { version: '' },
      python: v.python ?? { version: '' },
    }
  } catch {
    return { 'yt-dlp': { version: '' }, ffmpeg: { version: '' }, python: { version: '' } }
  }
}

async function writeSha(map: ShaMap): Promise<void> {
  await writeFile(SHA_PATH, JSON.stringify(map, null, 2) + '\n')
}

function sha256File(p: string): string {
  return createHash('sha256').update(Buffer.from(readFileSync(p))).digest('hex')
}

/** yt-dlp 固定版本（不用 latest：可复现构建，上游资产变更不会突然挂掉）。
 *  三平台 yt-dlp 均构建时拉取（仓库不存二进制），升级时改这里（三平台同步）；
 *  CI 二进制缓存按本文件哈希失效，会自动重新下载。
 *  本地更新：FORCE_BINARIES=1 pnpm run fetch:binaries */
const YTDLP_VERSION = '2026.08.19'
const YTDLP_DL = `https://github.com/yt-dlp/yt-dlp/releases/download/${YTDLP_VERSION}`

/** ffmpeg 固定版本（eugeneware/ffmpeg-static 项目：GitHub 分发、分架构、单文件 gzip）。
 *  BtbN 没有 macOS 构建、evermeet.cx 是外部小站且 getrelease 只有 Intel 版——都不用；
 *  Rust 侧 install_ffmpeg 运行时兜底仍走 BtbN/evermeet，仅作资源缺失时的自修复。 */
const FFMPEG_VERSION = 'b6.1.1'
const FFMPEG_DL = `https://github.com/eugeneware/ffmpeg-static/releases/download/${FFMPEG_VERSION}`

/** macOS 专用：捆绑 CPython 解释器（python-build-standalone，install_only 精简版，~24MB）。
 *  背景：官方 yt-dlp_macos 是 PyInstaller onefile，在这台 macOS 上每次启动被系统阻塞约 40s
 *  （实测），触发 VidGrab 60s 解析超时误杀与系统弹窗。macOS 改用捆绑 python 直跑 yt-dlp
 *  源码包（实测启动 <1s）。不依赖用户机器安装 python。
 *  macOS 系统自带 /usr/bin/python3 是 3.9.6，而 yt-dlp 2026.08.19 要求 ≥3.10，故捆绑 3.12。
 *  版本固定（发布 tag + CPython 版本），升级 = 改这里两处，CI 缓存按脚本哈希自动失效。 */
const PYTHON_BUILD_TAG = '20260901'
const PYTHON_VERSION = '3.12.14'
const PYTHON_DL = `https://github.com/astral-sh/python-build-standalone/releases/download/${PYTHON_BUILD_TAG}`

/** 选择 python-build-standalone 的 macOS 资产（x86_64 / arm64）。
 *  与 ffmpegAsset 同款逻辑：release 工作流经 TAURI_TARGET 指定目标架构。 */
function pythonAsset(): string {
  const t = process.env.TAURI_TARGET ?? ''
  const arm = t.includes('aarch64-apple-darwin') || (t === '' && process.platform === 'darwin' && process.arch === 'arm64')
  const arch = arm ? 'aarch64' : 'x86_64'
  return `cpython-${PYTHON_VERSION}+${PYTHON_BUILD_TAG}-${arch}-apple-darwin-install_only.tar.gz`
}

function pythonArchKey(): string {
  const t = process.env.TAURI_TARGET ?? ''
  const arm = t.includes('aarch64-apple-darwin') || (t === '' && process.platform === 'darwin' && process.arch === 'arm64')
  return arm ? 'darwin-arm64' : 'darwin-x64'
}

/** 单次下载超时：防止连接挂死把 CI 任务卡到 6 小时上限 */
const DOWNLOAD_TIMEOUT_MS = 10 * 60 * 1000

interface YtDlpTarget {
  url: string
  file: string
  /** unix 下是否补可执行权限 */
  exec: boolean
}

const YTDLP_TARGETS: Record<string, YtDlpTarget> = {
  win32: { url: `${YTDLP_DL}/yt-dlp.exe`, file: 'yt-dlp.exe', exec: false },
  darwin: { url: `${YTDLP_DL}/yt-dlp_macos`, file: 'yt-dlp', exec: true },
  linux: { url: `${YTDLP_DL}/yt-dlp_linux`, file: 'yt-dlp', exec: true },
}

/** 选择 ffmpeg-static 的资产名。
 *  release 工作流跨架构编译（arm64 runner 编 x86_64 mac 包）时通过 TAURI_TARGET
 *  环境变量传入目标三元组；本机构建（无该变量）按当前机器平台/架构选择。 */
function ffmpegAsset(): string {
  const t = process.env.TAURI_TARGET ?? ''
  if (t.includes('aarch64-apple-darwin')) return 'ffmpeg-darwin-arm64'
  if (t.includes('apple-darwin')) return 'ffmpeg-darwin-x64'
  if (t.includes('aarch64-unknown-linux') || t.includes('linux-arm64')) {
    return 'ffmpeg-linux-arm64'
  }
  if (t.includes('unknown-linux') || t.includes('linux')) return 'ffmpeg-linux-x64'
  if (t.includes('windows')) return 'ffmpeg-win32-x64'
  if (process.platform === 'darwin') {
    return process.arch === 'arm64' ? 'ffmpeg-darwin-arm64' : 'ffmpeg-darwin-x64'
  }
  if (process.platform === 'linux') {
    return process.arch === 'arm64' ? 'ffmpeg-linux-arm64' : 'ffmpeg-linux-x64'
  }
  return 'ffmpeg-win32-x64'
}

async function downloadOnce(
  url: string,
  destPath: string,
  minSize: number,
): Promise<number> {
  const resp: Response = await fetch(url, {
    redirect: 'follow',
    signal: AbortSignal.timeout(DOWNLOAD_TIMEOUT_MS),
  })
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status} ${resp.statusText}`)
  }
  const buf: Buffer = Buffer.from(await resp.arrayBuffer())
  // 过小说明拿到的不是二进制（如被网关劫持成错误页），避免坏文件混进安装包
  if (buf.length < minSize) {
    throw new Error(`文件过小(${buf.length} 字节，要求 ≥ ${minSize})，疑似下载不完整`)
  }
  const tmp = `${destPath}.download`
  await writeFile(tmp, buf)
  await rm(destPath, { force: true })
  await rename(tmp, destPath)
  return buf.length
}

/** 大文件网络抖动常见，重试 3 次 */
async function download(
  url: string,
  destPath: string,
  minSize: number,
  attempts = 3,
): Promise<number> {
  let lastErr: unknown
  for (let i = 1; i <= attempts; i++) {
    try {
      return await downloadOnce(url, destPath, minSize)
    } catch (err) {
      lastErr = err
      if (i < attempts) {
        const waitSec = i * 3
        console.log(`[fetch-binaries] 下载失败（第 ${i}/${attempts} 次），${waitSec}s 后重试...`)
        await new Promise((r) => setTimeout(r, waitSec * 1000))
      }
    }
  }
  throw lastErr
}

async function fetchYtDlp(target: YtDlpTarget): Promise<void> {
  if (process.platform === 'darwin') {
    // macOS：不再打包 PyInstaller onefile 的 yt-dlp_macos（启动被系统阻塞 ~40s），
    // 改打包官方 sdist 源码包，运行时由捆绑 python 直跑（见 downloader.rs / install.rs）
    return fetchYtDlpSource()
  }
  const dest = path.join(BIN_DIR, target.file)
  const lock = await readLock()
  if (existsSync(dest) && !process.env.FORCE_BINARIES) {
    if (lock['yt-dlp'] === YTDLP_VERSION) {
      console.log(`[fetch-binaries] ${target.file} 已存在且版本匹配（${YTDLP_VERSION}），跳过下载`)
      // 跳过下载也要记录 SHA256，供运行时安装器校验完整性
      // （darwin 已在上方 return，这里只可能 win32 / linux）
      const pk = process.platform === 'win32' ? 'win32' : 'linux'
      const shaMap = await readSha()
      shaMap['yt-dlp'] = { ...shaMap['yt-dlp'], version: YTDLP_VERSION, [pk]: sha256File(dest) }
      await writeSha(shaMap)
      return
    }
    console.log(`[fetch-binaries] ${target.file} 版本不是 ${YTDLP_VERSION}，重新下载`)
  }
  console.log(`[fetch-binaries] 下载 ${target.url} ...`)
  const size = await download(target.url, dest, 1024 * 1024)
  if (target.exec) {
    await chmod(dest, 0o755)
  }
  await updateLock('yt-dlp', YTDLP_VERSION)
  const pk = process.platform === 'win32' ? 'win32' : 'linux'
  const shaMap = await readSha()
  shaMap['yt-dlp'] = { ...shaMap['yt-dlp'], version: YTDLP_VERSION, [pk]: sha256File(dest) }
  await writeSha(shaMap)
  console.log(`[fetch-binaries] 完成: ${dest} (${(size / 1048576).toFixed(1)} MB)`)
}

/** 运行系统 tar 解压（macOS/Linux CI 与本机都自带；这里只在 darwin 分支使用） */
function runTar(args: string[]): void {
  const r = spawnSync('tar', args, { stdio: 'inherit' })
  if (r.status !== 0) {
    throw new Error(`tar ${args.join(' ')} 失败（exit ${r.status}）`)
  }
}

/** macOS：下载 yt-dlp 官方 sdist（yt-dlp.tar.gz），解压出 yt_dlp 包到 bin/yt-dlp-pkg/。
 *  sdist 的 SHA256 记录到 yt-dlp.darwin，供 install.rs 运行时兜底下载同款校验。 */
async function fetchYtDlpSource(): Promise<void> {
  const pkgDir = path.join(BIN_DIR, 'yt-dlp-pkg')
  const pkgMain = path.join(pkgDir, 'yt_dlp', '__main__.py')
  const lock = await readLock()
  if (!process.env.FORCE_BINARIES && lock['yt-dlp'] === YTDLP_VERSION && existsSync(pkgMain)) {
    console.log(`[fetch-binaries] yt-dlp 源码包已存在且版本匹配（${YTDLP_VERSION}），跳过下载`)
    return
  }
  const tarball = path.join(BIN_DIR, 'yt-dlp.tar.gz')
  console.log(`[fetch-binaries] 下载 yt-dlp 源码包 ${YTDLP_DL}/yt-dlp.tar.gz ...`)
  await download(`${YTDLP_DL}/yt-dlp.tar.gz`, tarball, 2 * 1024 * 1024)
  const sdistSha = sha256File(tarball)

  const extractDir = path.join(BIN_DIR, '.ytdlp-src-tmp')
  await rm(extractDir, { recursive: true, force: true })
  mkdirSync(extractDir, { recursive: true })
  await runTar(['xzf', tarball, '-C', extractDir])
  await rm(tarball, { force: true })
  // sdist 顶层目录名是 yt-dlp（不带版本号）
  const srcRoot = path.join(extractDir, 'yt-dlp')
  if (!existsSync(path.join(srcRoot, 'yt_dlp', '__main__.py'))) {
    throw new Error(`yt-dlp 源码包结构异常：未找到 ${path.join(srcRoot, 'yt_dlp')}`)
  }
  await rm(pkgDir, { recursive: true, force: true })
  mkdirSync(pkgDir, { recursive: true })
  await rename(path.join(srcRoot, 'yt_dlp'), path.join(pkgDir, 'yt_dlp'))
  await rm(extractDir, { recursive: true, force: true })

  await updateLock('yt-dlp', YTDLP_VERSION)
  const shaMap = await readSha()
  shaMap['yt-dlp'] = { ...shaMap['yt-dlp'], version: YTDLP_VERSION, darwin: sdistSha }
  await writeSha(shaMap)
  console.log(`[fetch-binaries] 完成: ${pkgDir} (sdist sha256 ${sdistSha.slice(0, 12)}…)`)
}

/** macOS：下载 python-build-standalone（install_only）解压到 bin/python/。
 *  压缩包 SHA256 记录到 python.darwin-*，供 install.rs 运行时兜底下载同款校验。 */
async function fetchPython(): Promise<void> {
  const asset = pythonAsset()
  const destDir = path.join(BIN_DIR, 'python')
  const pyMarker = path.join(destDir, 'bin', `python${PYTHON_VERSION.slice(0, PYTHON_VERSION.lastIndexOf('.'))}`)
  const lock = await readLock()
  const verKey = `${PYTHON_VERSION}+${PYTHON_BUILD_TAG}`
  if (!process.env.FORCE_BINARIES && lock['python'] === verKey && existsSync(pyMarker)) {
    console.log(`[fetch-binaries] 捆绑 python 已存在且版本匹配（${verKey}），跳过下载`)
    return
  }
  const url = `${PYTHON_DL}/${asset}`
  console.log(`[fetch-binaries] 下载捆绑 python ${asset} ...`)
  const tarball = path.join(BIN_DIR, asset)
  await download(url, tarball, 10 * 1024 * 1024)
  const pySha = sha256File(tarball)

  await rm(destDir, { recursive: true, force: true })
  // install_only tarball 顶层目录就是 python/
  await runTar(['xzf', tarball, '-C', BIN_DIR])
  await rm(tarball, { force: true })
  if (!existsSync(pyMarker)) {
    throw new Error(`捆绑 python 解压结构异常：未找到 ${pyMarker}`)
  }

  await updateLock('python', verKey)
  const shaMap = await readSha()
  shaMap.python = { ...shaMap.python, version: verKey, [pythonArchKey()]: pySha }
  await writeSha(shaMap)
  console.log(`[fetch-binaries] 完成: ${destDir} (${asset}, sha256 ${pySha.slice(0, 12)}…)`)
}

/** 下载 .gz 单文件并原地解压为最终二进制 */
async function fetchFfmpeg(): Promise<void> {
  const asset = ffmpegAsset()
  const dest = path.join(BIN_DIR, process.platform === 'win32' ? 'ffmpeg.exe' : 'ffmpeg')
  const lock = await readLock()
  if (existsSync(dest) && !process.env.FORCE_BINARIES) {
    if (lock['ffmpeg'] === FFMPEG_VERSION) {
      console.log(`[fetch-binaries] ${path.basename(dest)} 已存在且版本匹配（${FFMPEG_VERSION}），跳过下载`)
      // 跳过下载也要记录 SHA256，供运行时安装器校验完整性
      const assetKey = ffmpegAsset().replace(/^ffmpeg-/, '')
      const shaMap = await readSha()
      shaMap.ffmpeg = { ...shaMap.ffmpeg, version: FFMPEG_VERSION, [assetKey]: sha256File(dest) }
      await writeSha(shaMap)
      return
    }
    console.log(`[fetch-binaries] ${path.basename(dest)} 版本不是 ${FFMPEG_VERSION}，重新下载`)
  }
  const url = `${FFMPEG_DL}/${asset}.gz`
  console.log(`[fetch-binaries] 下载 ${url} ...`)
  const gzPath = `${dest}.gz`
  await download(url, gzPath, 10 * 1024 * 1024)

  const gzBuf: Buffer = await readFile(gzPath)
  const out = zlib.gunzipSync(gzBuf)
  if (out.length < 30 * 1024 * 1024) {
    throw new Error(`解压后仅 ${out.length} 字节，异常`)
  }
  await rm(gzPath, { force: true })
  await writeFile(dest, out)
  if (process.platform !== 'win32') {
    await chmod(dest, 0o755)
  }
  await updateLock('ffmpeg', FFMPEG_VERSION)
  const assetKey = ffmpegAsset().replace(/^ffmpeg-/, '')
  const shaMap = await readSha()
  shaMap.ffmpeg = { ...shaMap.ffmpeg, version: FFMPEG_VERSION, [assetKey]: sha256File(dest) }
  await writeSha(shaMap)
  console.log(`[fetch-binaries] 完成: ${dest} (${(out.length / 1048576).toFixed(1)} MB)`)
}

async function main(): Promise<void> {
  const ytdlp = YTDLP_TARGETS[process.platform]
  if (!ytdlp) {
    console.error(`[fetch-binaries] 不支持的平台: ${process.platform}`)
    process.exit(1)
  }

  mkdirSync(BIN_DIR, { recursive: true })
  await fetchYtDlp(ytdlp)
  if (process.platform === 'darwin') {
    await fetchPython()
  }
  try {
    await fetchFfmpeg()
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    console.error(`[fetch-binaries] ffmpeg 拉取失败: ${message}`)
    console.error('[fetch-binaries] 请检查网络后重试，或手动下载 ffmpeg 放到 src-tauri/bin/ 下')
    process.exit(1)
  }
}

main().catch((err: unknown) => {
  console.error('[fetch-binaries] 失败:', err instanceof Error ? err.message : String(err))
  process.exit(1)
})
