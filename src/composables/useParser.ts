// 视频解析域：URL 解析、格式轨道拆分与去重、清晰度/音质胶囊、选择与持久化。
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { formatSizeLabel, parseHeight } from '@/utils/format'
import type { DownloadMode, FormatInfo, TrackCapsule, VideoMetadata } from '@/types'

const LAST_URL_KEY = 'vidgrab-last-url'
const LAST_PARSE_KEY = 'vidgrab-last-parse'

// 传给后端的 cookie 参数：无 Cookie 文件时为 null（后端据此不加 --cookies）
function cookieArg(path: string): string | null {
  return path ? path : null
}

export function useParser(getCookieFile: () => string) {
  // 地址栏 URL 持久化：重新打开应用时恢复最后一次输入/解析的链接
  const url = ref(localStorage.getItem(LAST_URL_KEY) ?? '')
  watch(url, (v) => {
    try {
      localStorage.setItem(LAST_URL_KEY, v ?? '')
    } catch {
      // localStorage 不可用时忽略
    }
  })

  const loading = ref(false)
  const error = ref('')
  const metadata = ref<VideoMetadata | null>(null)
  const selVideo = ref('')
  const selAudio = ref('')
  const selectedCombo = ref('')
  const outputFileName = ref('')

  // 格式轨道拆分
  const hasVideoStream = (codec: string) => !!codec && codec !== 'none'
  const hasAudioStream = (codec: string) => !!codec && codec !== 'none'
  const isAudioOnly = (f: FormatInfo) =>
    f.is_audio_only || (!hasVideoStream(f.vcodec) && hasAudioStream(f.acodec))
  const hasVideo = (f: FormatInfo) => hasVideoStream(f.vcodec)
  const hasAudio = (f: FormatInfo) => hasAudioStream(f.acodec)

  // 同分辨率去重时的 codec 优先级：avc1 兼容性最好，优先
  function codecRank(vcodec: string): number {
    const c = vcodec.toLowerCase()
    if (c.startsWith('avc1')) return 0
    if (c.startsWith('hvc1') || c.startsWith('hevc')) return 1
    if (c.startsWith('av01')) return 2
    if (c.startsWith('vp9')) return 3
    return 9
  }

  // 视频轨：按分辨率降序，同高度只保留 codec 最优的一条
  const videoTracks = computed<FormatInfo[]>(() => {
    if (!metadata.value?.formats) return []
    const list = metadata.value.formats
      .filter((f) => {
        const note = (f.format_note || '').toLowerCase()
        if (note.includes('storyboard')) return false
        return hasVideo(f)
      })
      .sort((a, b) => parseHeight(b.resolution) - parseHeight(a.resolution))

    const seen = new Map<number, FormatInfo>()
    for (const f of list) {
      const h = parseHeight(f.resolution)
      const cur = seen.get(h)
      if (!cur) {
        seen.set(h, f)
      } else if (codecRank(f.vcodec) < codecRank(cur.vcodec)) {
        seen.set(h, f)
      } else if (codecRank(f.vcodec) === codecRank(cur.vcodec)) {
        // 同分辨率同优先级下，优先选带文件大小的版本，让 UI 能展示 size
        const curSize = cur.filesize ?? cur.filesize_approx
        const fSize = f.filesize ?? f.filesize_approx
        if (!curSize && fSize) seen.set(h, f)
      }
    }
    return [...seen.values()].sort(
      (a, b) => parseHeight(b.resolution) - parseHeight(a.resolution),
    )
  })

  // 音频轨：优先 mp4 兼容（aac/m4a），同组内按码率/体积降序。
  // 合并走 -c:a copy 零重编码，故默认倾向可直接 copy 的音频源；
  // 无兼容源时回退最高码率（后端会兜底重编码为 aac）。
  const audioTracks = computed<FormatInfo[]>(() => {
    if (!metadata.value?.formats) return []
    // 与后端 audio_mp4_compatible 对齐：这些编码可被 mp4 直接 copy（含裸 aac），
    // 另补 m4a 容器扩展名特判。用于把可直接 copy 的音频轨排到优先位。
    const MP4_AUDIO_CODECS = ['aac', 'mp4a', 'opus', 'mp3', 'ac3', 'eac3', 'flac', 'alac']
    const isMp4Audio = (f: FormatInfo) => {
      if ((f.ext || '') === 'm4a') return true
      const ac = (f.acodec || '').toLowerCase()
      return MP4_AUDIO_CODECS.some((c) => ac.includes(c))
    }
    return metadata.value.formats
      .filter((f) => {
        const note = (f.format_note || '').toLowerCase()
        if (note.includes('storyboard')) return false
        return isAudioOnly(f)
      })
      .sort((a, b) => {
        const aMp4 = isMp4Audio(a) ? 1 : 0
        const bMp4 = isMp4Audio(b) ? 1 : 0
        if (aMp4 !== bMp4) return bMp4 - aMp4
        const abrDiff = (b.abr || 0) - (a.abr || 0)
        if (abrDiff !== 0) return abrDiff
        return (b.filesize || 0) - (a.filesize || 0)
      })
  })

  const selectedVideoFormat = computed(() =>
    metadata.value?.formats.find((f) => f.format_id === selVideo.value),
  )
  const selectedAudioFormat = computed(() =>
    metadata.value?.formats.find((f) => f.format_id === selAudio.value),
  )
  const selectedVideoSize = computed(
    () =>
      selectedVideoFormat.value?.filesize ??
      selectedVideoFormat.value?.filesize_approx ??
      null,
  )
  const selectedAudioSize = computed(
    () =>
      selectedAudioFormat.value?.filesize ??
      selectedAudioFormat.value?.filesize_approx ??
      null,
  )

  // 下载模式由「选了哪些轨」推导
  const downloadMode = computed<DownloadMode>(() => {
    if (selVideo.value && !selAudio.value) return 'video'
    if (!selVideo.value && selAudio.value) return 'audio'
    return 'merge'
  })

  // 胶囊标签
  function videoCapsuleLabel(res: string): { label: string; sub: string } {
    const h = parseHeight(res)
    if (h >= 2160) return { label: '4K', sub: '原画' }
    if (h >= 1440) return { label: '2K', sub: '超清' }
    if (h >= 1080) return { label: '1080P', sub: '高清' }
    if (h >= 720) return { label: '720P', sub: '标清' }
    if (h >= 480) return { label: '480P', sub: '清晰' }
    if (h >= 360) return { label: '360P', sub: '流畅' }
    return { label: res, sub: '视频' }
  }

  function audioCapsuleLabel(abr: number | null): { label: string; sub: string } {
    if (abr && abr >= 256) return { label: '320K', sub: '高品质' }
    if (abr && abr >= 128) return { label: '192K', sub: '标准' }
    return { label: '128K', sub: '流畅' }
  }

  const videoCapsules = computed<TrackCapsule[]>(() =>
    videoTracks.value.map((f) => {
      const { label, sub } = videoCapsuleLabel(f.resolution)
      return { key: f.format_id, label, sub, size: formatSizeLabel(f).replace(/^· /, '') }
    }),
  )

  // 音频按码率档位分组，每档只留码率最高的一条
  const audioCapsules = computed<TrackCapsule[]>(() => {
    const groups = new Map<string, FormatInfo>()
    for (const f of audioTracks.value) {
      const { label } = audioCapsuleLabel(f.abr)
      const cur = groups.get(label)
      if (!cur || (f.abr || 0) > (cur.abr || 0)) groups.set(label, f)
    }
    const order = ['320K', '192K', '128K']
    return [...groups.entries()]
      .sort((a, b) => order.indexOf(a[0]) - order.indexOf(b[0]))
      .map(([label, f]) => ({
        key: f.format_id,
        label,
        sub: audioCapsuleLabel(f.abr).sub,
        size: formatSizeLabel(f).replace(/^· /, ''),
      }))
  })

  // 组合预设
  const comboOptions = [
    { label: '最佳画质', mode: 'merge' as const },
    { label: '1080P', mode: 'merge' as const, maxHeight: 1080 },
    { label: '720P', mode: 'merge' as const, maxHeight: 720 },
    { label: '仅音频', mode: 'audio' as const },
    { label: '仅视频', mode: 'video' as const },
  ]
  type ComboOption = (typeof comboOptions)[number]

  function pickHighestVideo(maxHeight?: number): FormatInfo | undefined {
    const list = maxHeight
      ? videoTracks.value.filter((f) => parseHeight(f.resolution) <= maxHeight)
      : videoTracks.value
    return list[0]
  }

  function selectCombo(opt: ComboOption) {
    selectedCombo.value = opt.label
    if (opt.mode === 'audio') {
      selVideo.value = ''
      selAudio.value = audioTracks.value[0]?.format_id ?? ''
    } else if (opt.mode === 'video') {
      // 优先含音轨的视频流（无需 merge 音频）
      const withAudio = videoTracks.value.find((f) => hasVideo(f) && hasAudio(f))
      selVideo.value = (withAudio ?? pickHighestVideo())?.format_id ?? ''
      selAudio.value = ''
    } else {
      selVideo.value = pickHighestVideo(opt.maxHeight)?.format_id ?? ''
      selAudio.value = audioTracks.value[0]?.format_id ?? ''
    }
  }

  function selectVideo(formatId: string) {
    selVideo.value = selVideo.value === formatId ? '' : formatId
  }
  function selectAudio(formatId: string) {
    selAudio.value = selAudio.value === formatId ? '' : formatId
  }

  // 解析成功后默认填入视频标题（不含扩展名，扩展名由后端按格式自动补），用户可改
  function fillDefaultOutputName() {
    if (!metadata.value) return
    const base = metadata.value.title.replace(/[\\/:*?"<>|]/g, '_')
    outputFileName.value = base
  }

  // 解析结果持久化：关闭应用后重开，恢复上次的解析卡片与选择
  function saveLastParse() {
    try {
      if (!metadata.value) {
        localStorage.removeItem(LAST_PARSE_KEY)
        return
      }
      localStorage.setItem(
        LAST_PARSE_KEY,
        JSON.stringify({
          metadata: metadata.value,
          selVideo: selVideo.value,
          selAudio: selAudio.value,
          selectedCombo: selectedCombo.value,
          outputFileName: outputFileName.value,
        }),
      )
    } catch {
      // 存储超限等忽略
    }
  }

  watch([metadata, selVideo, selAudio, selectedCombo, outputFileName], () =>
    saveLastParse(),
  )

  function restoreLastParse() {
    try {
      const raw = localStorage.getItem(LAST_PARSE_KEY)
      if (!raw) return
      const p = JSON.parse(raw)
      if (!p.metadata) return
      metadata.value = p.metadata
      selVideo.value = p.selVideo || ''
      selAudio.value = p.selAudio || ''
      selectedCombo.value = p.selectedCombo || ''
      outputFileName.value = p.outputFileName || ''
    } catch {
      // 恢复失败忽略
    }
  }

  async function parse() {
    if (!url.value) return
    if (!/https?:\/\//.test(url.value)) {
      error.value = '请输入有效的视频链接（需以 http:// 或 https:// 开头）'
      return
    }
    loading.value = true
    error.value = ''
    metadata.value = null
    selVideo.value = ''
    selAudio.value = ''
    selectedCombo.value = ''

    try {
      const info = await invoke<VideoMetadata>('parse_video', {
        url: url.value,
        cookieSource: cookieArg(getCookieFile()),
      })
      metadata.value = info
      fillDefaultOutputName()
      selectCombo(comboOptions[0])
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  // 收起解析卡片（保留 URL，方便换链接重新解析）
  function dismiss() {
    metadata.value = null
    selVideo.value = ''
    selAudio.value = ''
    selectedCombo.value = ''
    error.value = ''
  }

  // 只暴露外部真正消费的项：videoTracks / audioTracks / comboOptions /
  // selectedCombo 均为内部派生或持久化用，不外泄以免接口被误用
  return {
    url,
    loading,
    error,
    metadata,
    selVideo,
    selAudio,
    outputFileName,
    videoCapsules,
    audioCapsules,
    selectedVideoSize,
    selectedAudioSize,
    downloadMode,
    canDownload: computed(() => !!selVideo.value || !!selAudio.value),
    parse,
    dismiss,
    restoreLastParse,
    selectVideo,
    selectAudio,
  }
}
