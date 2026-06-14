# VRCaster 接口契约（多 agent 实现的权威契约）

所有 Rust 模块与前端组件必须严格遵守本契约的数据结构、命令签名、事件名。
共享数据结构定义在 `src-tauri/src/types.rs`（serde 序列化）与 `src/lib/types.ts`（TS 对应）。

## 1. 共享数据结构（Rust serde / TS 镜像）

### MediaInfo（媒体分析结果）
```
MediaInfo {
  path: String,
  duration_secs: f64,
  width: u32, height: u32,
  fps: f64,
  is_hdr: bool,
  color_transfer: Option<String>,   // smpte2084 / arib-std-b67 / bt709 ...
  size_bytes: u64,
  audio_tracks: Vec<AudioTrack>,
  subtitle_tracks: Vec<SubtitleTrack>,
}
AudioTrack { index: u32, codec: String, channels: u32, language: Option<String>, title: Option<String> }
SubtitleTrack { index: u32, codec: String, language: Option<String>, title: Option<String> }
```

### EncodeParams（用户可调编码参数）
```
EncodeParams {
  resolution: Resolution,            // "p720" | "p1080" | "p1440" | "p2160"
  rate_mode: RateMode,               // 见下
  speed: Speed,                      // "fast" | "medium" | "slow"
  hwaccel: bool,                     // GPU 硬解
  tonemap: Tonemap,                  // "hable" | "mobius" | "reinhard"
  audio_track_index: u32,
  audio_bitrate_k: u32,              // 128 | 192 | 256
  subtitle: SubtitleChoice,          // 见下
}
RateMode = TargetSize { gb: f64 } | Quality { crf: u32 }
SubtitleChoice = Embedded { index: u32 } | External { path: String } | None
```
锁死预设（不暴露）：H.264 main、bframes=0、yuv420p、faststart、AAC 立体声、HDR 自动→SDR。

### SampleSpec / SampleResult（样片标定）
```
SampleSpec { start_secs: f64, duration_secs: f64 }   // 默认：start=duration/2-60, dur=120
SampleResult {
  output_path: String,
  size_bytes: u64,
  avg_bitrate_k: u32,
  elapsed_secs: f64,
  full_estimate: FullEstimate,
}
FullEstimate { size_bytes: u64, bitrate_k: u32, elapsed_secs: f64, suggested: Option<EncodeParams> }
```

### ServerProfile（服务器配置；密码/口令存 keyring，不在结构里）
```
ServerProfile {
  id: String, name: String,
  host: String, port: u16, username: String,
  auth_kind: AuthKind,               // "password" | "privatekey"
  private_key_path: Option<String>,  // privatekey 时
  remote_dir: String,                // 如 /var/www/html
  url_prefix: String,                // 如 http://host 或 http://video.domain
  access_key: String,                // ?key= 的值
}
AuthKind = "password" | "privatekey"
// 密钥/口令通过 keyring 存取，service="vrcaster", account=profile.id
```

### QueueItem / 模式
```
QueueItem {
  id: String,
  media: MediaInfo,
  params: EncodeParams,
  status: ItemStatus,                // "pending"|"encoding"|"encoded"|"uploading"|"done"|"error"
  encode_progress: f32,             // 0..100
  upload_progress: f32,
  result_url: Option<String>,
  error: Option<String>,
}
QueueMode = "pipeline" | "parallel_encode_serial_upload" | "parallel_all"
```

## 2. Tauri 命令签名（lib.rs 注册，前端 invoke）
```
analyze_media(path: String) -> Result<MediaInfo>
default_sample_spec(duration_secs: f64) -> SampleSpec
encode_sample(media: MediaInfo, params: EncodeParams, spec: SampleSpec) -> Result<SampleResult>
start_encode(item_id: String, media: MediaInfo, params: EncodeParams) -> Result<String>   // 返回成品路径；进度走事件
start_upload(item_id: String, local_path: String, profile_id: String) -> Result<String>   // 返回最终 URL；进度走事件
run_queue(items: Vec<QueueItem>, mode: QueueMode, profile_id: String) -> Result<()>

list_profiles() -> Result<Vec<ServerProfile>>
save_profile(profile: ServerProfile, secret: Option<String>) -> Result<()>   // secret 入 keyring
delete_profile(id: String) -> Result<()>
generate_access_key() -> String
check_remote_space(profile_id: String) -> Result<RemoteSpace>                 // { free_bytes, total_bytes }
deploy_nginx(profile_id: String) -> Result<String>                           // 返回部署日志

ensure_ffmpeg() -> Result<FfmpegPaths>                                       // { ffmpeg, ffprobe }；首次下载
get_settings() -> Result<AppSettings>
set_settings(settings: AppSettings) -> Result<()>
export_config(path: String) -> Result<()>
import_config(path: String) -> Result<()>
```

## 3. 事件（emit 到前端）
```
"progress"  -> ProgressEvent { item_id: String, stage: "encode"|"upload", percent: f32, speed: String, eta_secs: Option<f64> }
"log"       -> LogEvent { item_id: Option<String>, level: "info"|"warn"|"error", message: String }
"ffmpeg-download" -> DownloadEvent { percent: f32, message: String }
```

## 4. 模块职责与对外函数（各 agent 实现）
- `types.rs`：上述所有结构 + serde 派生（由地基提供）
- `media.rs`：`analyze(path, ffprobe) -> Result<MediaInfo>`
- `encode.rs`：`build_filter(params, media) -> String`；`run(app, item_id, media, params, ffmpeg, output) -> Result<()>`（解析 -progress，emit progress）；`run_sample(...) -> Result<SampleResult>`
- `subtitle.rs`：`extract_track(input, index, out) `；`extract_fonts(input, dir)`；供 encode 的字幕滤镜与字体目录
- `upload.rs`：`upload(app, item_id, local, profile, secret) -> Result<String>`（russh-sftp，断点续传：首次 put，续传按远程大小 seek，循环至大小一致；emit progress）；`remote_space(profile) -> RemoteSpace`
- `server.rs`：`gen_nginx_conf(profile) -> String`；`deploy(profile, secret) -> Result<String>`（SSH：装 nginx→写配置→nginx -t→reload）
- `config.rs`：profile 增删查 + keyring 存取 + AppSettings + 导出/导入
- `queue.rs`：`run(app, items, mode, profile, secret)`（pipeline：压制串行 + 上传与下个压制重叠；另两种模式）
- `ffmpeg.rs`：`ensure(app) -> FfmpegPaths`（检测本地→否则下载 gyan.dev 构建 zip 解压到 app data）

## 5. 关键实现要点（吸取手动流程的教训）
- 滤镜链（HDR→SDR + 缩放 + 字幕硬编，顺序固定）：
  `zscale=w=W:h=H:t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=tonemap=<algo>:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p,subtitles=<escaped ass path>`
  SDR 源跳过 tonemap 段，仅 scale + 字幕。
- 编码固定参数：`-c:v libx264 -profile:v main -pix_fmt yuv420p -x264-params keyint=48:min-keyint=48:bframes=0:scenecut=0 -fps_mode cfr -movflags +faststart -dn`
- 目标大小→码率：`bitrate = target_bytes*8/duration - audio_bitrate`，ABR（-b:v + -maxrate + -bufsize）
- GPU 硬解：`-hwaccel auto`（探测失败回退软解）
- 上传断点续传：首次 put、续传 seek（避开 sftp reput 对不存在文件失败的坑），循环重试至本地/远程字节一致
- 字幕路径在 subtitles 滤镜里需转义（Windows 冒号反斜杠）；优先把字幕提取为工作目录内 .ass 用相对名
- key 鉴权 URL：`{url_prefix}/{filename}?key={access_key}`
