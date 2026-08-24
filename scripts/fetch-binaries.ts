// 构建期拉取当前平台的 yt-dlp 与 ffmpeg 到 src-tauri/bin/，供 tauri 打包为应用资源。
// - 已存在则跳过（离线/重复构建不重新下载）；设 FORCE_BINARIES=1 强制重新拉取
// - Windows 的 yt-dlp.exe 随仓库提交，通常直接命中跳过
// - 体积大的二进制（yt-dlp unix 版、ffmpeg 全平台）不入库，各构建机首次构建时自动下载
//   （用 node 原生 TS 类型剥离直接运行：node scripts/fetch-binaries.ts，需 Node ≥22.18）
//
// 两个二进制都只取 ffmpeg 单体、不打 ffprobe（全项目只用 ffmpeg 做分轨合并）。
// 来源均在 GitHub：CI 同机房直连 + 本脚本重试/超时 + 工作流 actions/cache 三重保障。
import { existsSync, mkdirSync } from 'node:fs'
import { chmod, readFile, rename, rm, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import zlib from 'node:zlib'

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

/** yt-dlp 固定版本（不用 latest：可复现构建，上游资产变更不会突然挂掉）。
 *  必须与仓库里提交的 Windows 版 bin/yt-dlp.exe 同版本；
 *  升级时改这里（三平台同步），CI 二进制缓存按本文件哈希失效，会自动重新下载。
 *  本地同步更新 Windows 版：FORCE_BINARIES=1 pnpm run fetch:binaries 后 git add bin/yt-dlp.exe */
const YTDLP_VERSION = '2026.08.19'
const YTDLP_DL = `https://github.com/yt-dlp/yt-dlp/releases/download/${YTDLP_VERSION}`

/** ffmpeg 固定版本（eugeneware/ffmpeg-static 项目：GitHub 分发、分架构、单文件 gzip）。
 *  BtbN 没有 macOS 构建、evermeet.cx 是外部小站且 getrelease 只有 Intel 版——都不用；
 *  Rust 侧 install_ffmpeg 运行时兜底仍走 BtbN/evermeet，仅作资源缺失时的自修复。 */
const FFMPEG_VERSION = 'b6.1.1'
const FFMPEG_DL = `https://github.com/eugeneware/ffmpeg-static/releases/download/${FFMPEG_VERSION}`

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
  const dest = path.join(BIN_DIR, target.file)
  const lock = await readLock()
  if (existsSync(dest) && !process.env.FORCE_BINARIES) {
    if (lock['yt-dlp'] === YTDLP_VERSION) {
      console.log(`[fetch-binaries] ${target.file} 已存在且版本匹配（${YTDLP_VERSION}），跳过下载`)
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
  console.log(`[fetch-binaries] 完成: ${dest} (${(size / 1048576).toFixed(1)} MB)`)
}

/** 下载 .gz 单文件并原地解压为最终二进制 */
async function fetchFfmpeg(): Promise<void> {
  const asset = ffmpegAsset()
  const dest = path.join(BIN_DIR, process.platform === 'win32' ? 'ffmpeg.exe' : 'ffmpeg')
  const lock = await readLock()
  if (existsSync(dest) && !process.env.FORCE_BINARIES) {
    if (lock['ffmpeg'] === FFMPEG_VERSION) {
      console.log(`[fetch-binaries] ${path.basename(dest)} 已存在且版本匹配（${FFMPEG_VERSION}），跳过下载`)
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
