# AGENTS.md

给 AI 编码助手的项目须知。人类文档见 [README.md](./README.md)。

## 项目概述

VidGrab：基于 Tauri 2 的视频下载器。前端 Vue 3 + TypeScript（Vite + UnoCSS），后端 Rust
（`src-tauri/`），通过调用 yt-dlp 解析/下载、ffmpeg 合并。支持 Windows / macOS / Linux。
UI 与错误信息使用中文，新代码保持一致。

## 常用命令

```bash
pnpm tauri dev        # 开发
pnpm run build        # 前端构建（含 app + node 两轮 vue-tsc 类型检查）
pnpm test:run         # 前端单测（vitest，一次性跑完）
pnpm tauri build      # 当前平台安装包（Tauri 不能交叉打包）
cargo test            # 在 src-tauri/ 下执行；全新检出前需先 pnpm run build（generate_context! 需要 dist/）
pnpm run fetch:binaries  # 拉取当前平台的 yt-dlp 独立版到 src-tauri/bin/
```

## 硬性约束（违反会产生真实 bug，多数已踩过坑）

1. **`src-tauri/bin/yt-dlp.exe` 必须是官方独立版**（PyInstaller 打包，约 17MB）。
   禁止使用 pip 生成的启动器存根或 python zipapp——目标机器没有 Python，会解析失败
   （历史上发生过：108KB 存根在开发机靠本地 Python 蒙混过关，其他机器全挂）。
2. **所有子进程 spawn 必须加 CREATE_NO_WINDOW**。用现成的 `proc::hide_window`（std）
   或 `proc::hide_window_tokio`（tokio）。release 版是 `windows_subsystem = "windows"` 的 GUI 程序，
   裸 spawn 控制台程序会弹出黑色 cmd 窗口。创建标志常量 `CREATE_NO_WINDOW` 也定义在 `proc.rs`（仅 Windows）。
3. **平台差异配置放 `src-tauri/tauri.{windows,macos,linux}.conf.json`**（构建时自动与
   `tauri.conf.json` 合并），不要写进基础配置。当前拆分：bundle targets 与 resources 按平台不同。
4. **ffmpeg 随安装包分发（只打 ffmpeg，不打 ffprobe——全项目只用 ffmpeg 合并，ffprobe 是死重）**。
   由 `scripts/fetch-binaries.ts` 在构建期拉到 `src-tauri/bin/`，不入库（体积大）。
   三平台统一用 eugeneware/ffmpeg-static 固定版本（GitHub 分发、分架构、单文件 gzip）；
   **禁止改回 evermeet.cx 的 getrelease**（那是纯 Intel 单架构，Apple Silicon 机器上会挂）
   和 BtbN（无 macOS 构建）。BtbN/evermeet 仅存在于 Rust 运行时兜底 `install_ffmpeg` 中。
5. **跨平台二进制查找走统一入口**：`downloader::find_bundled_binary` / `ytdlp_candidates`
   （exe 目录 → resource_dir/app_data/bin → 环境变量 → 常见路径 → PATH）。macOS 资源在
   `Contents/Resources` 与可执行文件不同级，Linux deb 在 `/usr/lib` 下，不能只查 exe 同目录。
   unix 下找到的二进制要过 `ensure_exec`/`chmod_exec` 补可执行权限。
6. **运行时安装目录**：Windows 用 exe 同目录；macOS/Linux 的 exe 目录可能只读，
   必须用 `bin_install_dir`（app data 目录）。
7. **换行符一律 LF**（`.gitattributes` 强制），Windows 专属脚本（bat/ps1）除外。不要转换既有文件。
8. **Node 脚本用 TS 原生运行**（`node scripts/xxx.ts`，Node ≥22.18 类型剥离），不引入 tsx；
   脚本里避免 enum/namespace 等无法类型剥离的语法。`tsconfig.node.json` 覆盖 `scripts/` 与
   `commitlint.config.ts`，会随 `pnpm run build` 一起做类型检查。
9. **Rust 工具链由 rust-toolchain.toml 固定**（本地 rustup 与 CI 都遵循，CI 工作流从该文件
   读取版本）。升级 = 改文件里的 channel 一处，改完跑 `pnpm run build` + `cargo test` 验证。
   同理 yt-dlp/ffmpeg 版本固定在 `scripts/fetch-binaries.ts` 常量里——所有"什么时候升级"
   都由显式改文件触发，不存在自动漂移。
10. **子进程管道输出必须 lossy 读取 + 强制 PYTHONIOENCODING=utf-8**（见 `proc.rs` 的
    `read_line_lossy` / `force_utf8_env`）。yt-dlp 是 Python 程序，Windows 下 stdout 为
    管道时默认按本地编码（中文系统 GBK）输出；Rust 若按 UTF-8 严格解码，读取任务会因
    InvalidData 退出并关闭管道，子进程下次写进度即得到 Windows 断管道错误（errno 22），
    表现为「下载失败 [Errno 22] Invalid argument」。此 bug 已实际发生并被复现验证过。
11. **跨平台 cfg 分支必须在 CI 三平台都编译**：`proc.rs` 的 `attach_kill_on_close_job`、
    `CREATE_NO_WINDOW` 等仅 `#[cfg(windows)]` 定义，import 与调用点必须同带 cfg。GitHub Actions
    在 Windows/macOS/Linux 各跑一遍 `cargo clippy --all-targets -- -D warnings`（见 ci.yml），
    **非 Windows 平台编译失败（如漏写 cfg 导致 `E0432 unresolved import`）会在 push 时直接让 CI 红**，
    不要只在本地 Windows 验证。
12. **后端按域拆分模块，单文件控制在 ~500 行内**：`lib.rs` 只放 Tauri 命令入口与少量胶水
    （`get_default_download_dir` / `get_app_version` 等）；解析 `meta.rs`、链接清洗 `url.rs`、
    二进制查找与命令封装 `downloader.rs`、单轨执行与合并 `track.rs`、进度解析 `progress.rs`、
    进程控制原语 `proc.rs`、状态容器 `state.rs`、历史 `history.rs`、运行时安装 `install.rs`、
    文件名工具 `naming.rs`。命令函数用 `#[tauri::command]` 在各模块就地定义，`lib.rs` 的
    `invoke_handler!` 统一注册——不要再把所有命令堆回 `lib.rs`。
13. **`.part` 碎片只在「暂停」时保留，其余放弃/重来操作一律清理**：暂停 → 终止进程但留
    `.part`（供「继续下载」断点续传）；取消 / 删除 / 重新下载 都调 `clear_task_part`（或
    `delete_task` / `redownload_cleanup`）清掉 `.part` 与 `.part-Frag` 碎片。清理按
    `{safe_title}.` 前缀 + `.part`/`.part-Frag` 后缀匹配，不会误删其他任务或成品
    （`{prefix}.mp4`）。前端「重新下载」(`restartTask` / `redownloadFromRecord`) 语义是
    「换一份新的」，必须从头下而非续传，改动时勿让 yt-dlp 续上旧 `.part`。

## 提交规范

Conventional Commits，husky + CI 双重强制：
`<type>(<scope>?): <subject>`，type 限定 feat/fix/docs/style/refactor/perf/test/build/ci/chore/revert。
例：`feat: 支持macOS和Linux`、`fix(parser): 空指针`。不合规直接被拒。

## 发版流程

改版本号（`src-tauri/tauri.conf.json` 与 `src-tauri/Cargo.toml` 同步）→ 打 `v*` tag 推送 →
`.github/workflows/release.yml` 在三平台矩阵构建 → 产物进草稿 Release → 人工确认后 Publish。
tag 必须等于 `v` + 应用版本，工作流会校验。

## 样式分层规范

前端样式分四层，自上而下优先级递增，按场景选用：

| 场景 | 做法 |
|---|---|
| 组件独有样式（仅本组件用） | `<style scoped>` |
| 跨组件通用语义样式 | UnoCSS `shortcuts`（首选）或 `src/style.css` |
| 常规布局 / 间距 / 颜色 / 字号 | UnoCSS 原子类（`flex`、`mt-4`、`text-v-text` 等） |
| 复杂动画、渐变 + 阴影组合、伪元素 | 手写 CSS（scoped 或 `style.css`） |

约定与权衡：

1. **跨组件通用语义类一律用 `shortcuts`，不要各组件各写一遍手写 CSS。**
   `shortcuts` 由 UnoCSS 生成唯一一份、天然复用，且**可被原子类覆盖**（调用处可
   `class="set-hint mt-4"` 微调）。`style.css` 里的类无法被原子类覆盖（优先级问题），
   会反逼调用方继续写手写 CSS，与"原子类优先"原则相悖。当前 `set-label` / `set-hint` /
   `set-browse` / `set-input` / `set-save` 均已收敛到 `uno.config.ts` 的 `shortcuts`。
2. **伪类/状态变体尽量用 UnoCSS 变体表达**（`hover:` / `focus:` / `placeholder:` /
   `active:`），避免为了 hover 写手写 CSS。条件是显式状态类（如"已保存""确认删除"）才
   定义成独立 shortcut（如 `set-save-saved`、`set-browse-confirm`），在模板用 `:class` 绑定。
3. **`style.css` 只放真正的全局基础与 design token**（`:root` 变量、滚动条、`#app` 基础、
   过渡动画）。零引用的废弃类及时删，不要"保留给未来使用"。
4. **复杂动画、玻璃模糊（backdrop-filter）、多属性渐变阴影组合**原子类不便表达，允许手写
   CSS，放在对应组件 `<style scoped>`。这类是分层规范的例外，不是偷懒借口。
5. 改动样式前先比对各组件现有同名定义，取"出现次数最多且与 design token 一致"的为准，
   视觉零变化收敛；差异项记入注释。

## 验证要求

改动完成后至少跑通：`pnpm test:run` + `pnpm run build` + `cargo test` + `cargo clippy --all-targets -- -D warnings`
（前端 `pnpm test:run` 为 vitest 单测；在 `src-tauri/` 下，所有非 `#[ignore]` 的单测全过为基线；`#[ignore]` 的集成测试需网络，默认不跑；
clippy 零警告为门槛，CI 三平台均强制执行）。
