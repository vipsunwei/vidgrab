# Changelog

本项目的所有显著变更记录于此，格式基于 Keep a Changelog，
分组来自 Conventional Commits 提交规范。

## [0.0.2] - 2026-09-06

### ✨ 新功能
- Cookie 按站点分组管理并移除浏览器读取方式，确立样式分层规范
- 安装器二进制加 SHA256 校验

### 🐛 问题修复
- 优先用数值 duration 修复部分站点时长缺失；解析错误提示支持手动关闭
- **url**: 域名精确匹配，修复 netflix.com 误判为 Twitter/X
- 修复下载文件匹配、清理与进程控制的边界问题
- **download**: 统一临时碎片清理并修复单轨取消/失败后 .part 残留
- **track**: 精确匹配 416 断点失效，避免裸数字误判
- **download**: 合并优先零重编码 aac/m4a 音频
- **download**: 解析异步化并修 unix 树杀
- **cookies**: 限制 add_cookie_store 读取路径
- **parser**: 前端 isMp4Audio 对齐后端编码集
- **merge**: 给 ffmpeg 合并阶段加超时兜底
- **parser**: 音轨排序只认后端真能 copy 的 aac/mp4a
- 修 clippy 告警
- **ui**: 长标题显示省略号，不再挤出收起按钮
- **parser**: 默认音频选中与胶囊列表对齐

### 💄 样式
- 注释统一为 //，类型文件用 /** */

### ♻️ 重构
- **downloader**: 移除 cookie 的 none 判断并补充单测
- 删除 greet 死代码

### ✅ 测试
- 引入 vitest 并补充 detectPlatform 单测

### 📝 文档
- README 改为用户向，开发者内容移至 CONTRIBUTING.md

### 📦 构建
- 前端单测纳入 CI 与类型检查

### 🔧 杂项
- 压缩 app-icon 体积
- 忽略 .codebuddy/ 工作区数据
- 加一键发版脚本并完善文档链接
- 发版脚本推送前自动探测本地代理
<!-- 由 git-cliff 自动生成，请勿手动编辑 -->
