# VidGrab

基于 Tauri 2 + Vue 3 的视频下载器，支持 Windows / macOS / Linux。

## 开发

```bash
pnpm install
pnpm tauri dev
```

## 构建

各平台本机构建对应安装包（Tauri 不支持交叉打包）：

```bash
pnpm tauri build
```

| 平台 | 产物 |
| --- | --- |
| Windows | `src-tauri/target/release/bundle/nsis/*.exe`（NSIS） |
| macOS | `src-tauri/target/release/bundle/dmg/*.dmg` + `macos/*.app` |
| Linux | `src-tauri/target/release/bundle/deb/*.deb` + `appimage/*.AppImage` |

### 发版（CI 出三平台安装包）

安装包统一由 GitHub Actions 出（`.github/workflows/release.yml`），本地不必备齐三台机器：

1. 改版本号：`src-tauri/tauri.conf.json` 与 `src-tauri/Cargo.toml` 保持一致；
2. 打 tag 推送（tag 必须等于 `v` + 应用版本，工作流会校验）：

   ```bash
   git tag v0.0.1
   git push origin v0.0.1
   ```

3. 四个矩阵任务并行构建：Windows NSIS / macOS dmg（Apple Silicon + Intel）/ Linux deb + AppImage；
4. Release 正文由 [git-cliff](https://git-cliff.org) 按 `cliff.toml` 从提交记录自动生成；
5. 产物自动挂到一个**草稿 Release**，检查附件无误后手动点 Publish 发布。

### Changelog

- `CHANGELOG.md` 由 git-cliff 依据 Conventional Commits 生成，发版后手动刷新：
  `git-cliff -o CHANGELOG.md`（Windows 侧可从 GitHub Releases 下载二进制）。
- Release 工作流自动用 `git-cliff --latest --strip all` 生成草稿 Release 的正文，无需手动写。

### 内置二进制说明

- `src-tauri/bin/yt-dlp.exe`（Windows）：**随仓库提交**，打包进安装包，目标机器无需 Python。
  必须使用官方独立版（PyInstaller 打包，约 17MB），不要用 pip 生成的启动器存根或 python zipapp。
- `src-tauri/bin/yt-dlp`（macOS/Linux）与 **ffmpeg（三平台）**：体积大不入库，由
  `scripts/fetch-binaries.ts` 在 `beforeBuildCommand` 阶段自动下载（已存在则跳过，
  `FORCE_BINARIES=1` 强制重拉）。ffmpeg 只打单体（不打 ffprobe）。
- 构建期来源全部在 GitHub：yt-dlp 固定版本（脚本常量 `YTDLP_VERSION`，须与提交的
  Windows 版 exe 同版本）；ffmpeg 用 ffmpeg-static 项目固定版本（常量 `FFMPEG_VERSION`，
  按 `TAURI_TARGET` 选架构）。升级：改常量 → `FORCE_BINARIES=1 pnpm run fetch:binaries`
  → 提交更新后的 `bin/yt-dlp.exe`。CI 对二进制做了 actions/cache，命中后构建不依赖外部下载源。
- `install_ffmpeg` 运行时安装保留为兜底（打包资源缺失时应用内自修复），其源为
  Windows/Linux BtbN（zip / tar.xz）、macOS evermeet.cx。yt-dlp 仅依赖构建期打包的
  独立版，无运行时兜底安装路径。

平台差异配置在 `src-tauri/tauri.{windows,macos,linux}.conf.json`，构建时自动与 `tauri.conf.json` 合并。

## 提交规范（强制）

使用 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)，格式：

```
<type>(<scope>?): <subject>

例：feat: 支持macOS和Linux
    fix(parser): 空指针崩溃
```

type 限定：`feat` `fix` `docs` `style` `refactor` `perf` `test` `build` `ci` `chore` `revert`。

三层强制：

1. **husky commit-msg hook**：提交时本地校验（`pnpm install` 后自动生效）。
2. **CI 校验**（`.github/workflows/ci.yml`）：push/PR 时再次校验，防止 `--no-verify` 绕过。
3. CI 同时在 Windows / macOS / Linux 三平台跑 `cargo test` 与 `cargo clippy --all-targets
   -- -D warnings`，保证跨平台代码可编译、可通过 lint（`-[D] warnings` 让死代码/未使用 import
   等问题直接失败，不会在 CI 悄悄累积）。

## 目录结构要点

```
AGENTS.md                     # AI 编码助手项目须知（硬性约束与坑）
CHANGELOG.md / cliff.toml     # 变更日志与 git-cliff 生成配置
src-tauri/src/lib.rs          # Tauri 命令注册入口（invoke_handler!），仅含少量胶水命令
src-tauri/src/downloader.rs   # yt-dlp/ffmpeg 二进制查找、命令封装、JSON 抓取
src-tauri/src/meta.rs         # yt-dlp JSON 解析与缩略图处理
src-tauri/src/cookies.rs       # Cookie 固定存储（Netscape 格式），按站点组管理（add/remove/clear/status/path 命令）
src-tauri/src/url.rs          # 链接清洗与平台识别（extract_first_url / detect_platform）
src-tauri/src/track.rs        # 单轨下载执行、断点续传重试、ffmpeg 合并
src-tauri/src/progress.rs     # 进度行解析（yt-dlp / ffmpeg）与事件负载/回传
src-tauri/src/proc.rs         # 进程控制原语（隐藏窗口、lossy 读取、PID 树击杀、跨平台 cfg）
src-tauri/src/state.rs        # 运行时状态容器（任务表、暂停/取消集合、运行代号）
src-tauri/src/download.rs     # 下载编排：start_download / pause / cancel / delete_task / clear_task_part 命令
src-tauri/src/history.rs      # 历史与文件持久化（load/save_history、load/save_tasks、register_task_output、delete_file、redownload_cleanup）
src-tauri/src/install.rs      # 运行时依赖安装兜底（install_ffmpeg）
src-tauri/src/naming.rs       # 输出文件名工具
scripts/fetch-binaries.ts    # 构建期拉取平台 yt-dlp/ffmpeg 二进制
.husky/commit-msg             # 提交信息校验 hook
```
