# VRCaster

把视频一键压制成 **VRChat 兼容格式**并上传到自己的服务器，产出可直接播放的带 key URL。

## 特性

- **VRChat 兼容预设**（锁死）：H.264 main / **无 B 帧** / yuv420p / faststart / AAC 立体声
- **HDR → SDR**：自动检测 HDR10/HLG，tonemap（hable / mobius / reinhard）
- 分辨率 720p / 1080p / 1440p / 4K；**目标大小反算码率** 或 CRF 质量档
- 字幕：内嵌轨硬编 / 外挂 `.ass`·`.srt` / 自动字体回退
- 音轨选择 + 降混立体声（码率可调）
- **样片标定**：试压中间一段，反推全片大小 / 码率 / 耗时
- **完整 ffmpeg 进度**：帧 / fps / 码率 / 已编码时间 / 速度 / ETA
- 批量队列：流水线 / 并发压制+顺序上传 / 全并发
- 压制 **停止 / 暂停 / 继续**
- **SFTP 断点续传** + **nginx key 鉴权一键部署**
- 凭据存系统密钥环；多服务器 profile；配置导出导入
- 成品输出到 `源目录/outs/`

## 技术栈

Tauri 2 + Svelte 4 + TypeScript；ffmpeg（优先用系统 PATH，找不到才下载）。

## 开发

```bash
npm install
npm run tauri dev      # 开发
npm run tauri build    # 打包
```

需要 Rust、Node、以及 Windows 上的 MSVC 构建工具。

## 说明

- ffmpeg/ffprobe 优先使用系统 PATH 中已安装的版本，缺失时才下载。
- 服务器密码 / 私钥口令存于操作系统凭据管理器，**不入库、不随配置导出**。
- 当前面向 Windows；Tauri 跨平台，后续可扩展。
