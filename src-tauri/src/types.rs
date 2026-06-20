//! 共享数据结构。前端镜像见 src/lib/types.ts，权威定义见 docs/contract.md。
//! serde 统一 camelCase，与 TS 对应。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrack {
    pub index: u32,
    pub codec: String,
    pub channels: u32,
    pub language: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleTrack {
    pub index: u32,
    pub codec: String,
    pub language: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub path: String,
    pub duration_secs: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub is_hdr: bool,
    pub color_transfer: Option<String>,
    pub size_bytes: u64,
    pub audio_tracks: Vec<AudioTrack>,
    pub subtitle_tracks: Vec<SubtitleTrack>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Resolution {
    P720,
    P1080,
    P1440,
    P2160,
}

impl Resolution {
    /// 返回目标宽高（16:9）。
    pub fn dims(self) -> (u32, u32) {
        match self {
            Resolution::P720 => (1280, 720),
            Resolution::P1080 => (1920, 1080),
            Resolution::P1440 => (2560, 1440),
            Resolution::P2160 => (3840, 2160),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Speed {
    Fast,
    Medium,
    Slow,
}

impl Speed {
    pub fn preset(self) -> &'static str {
        match self {
            Speed::Fast => "fast",
            Speed::Medium => "medium",
            Speed::Slow => "slow",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tonemap {
    Hable,
    Mobius,
    Reinhard,
}

impl Tonemap {
    pub fn as_str(self) -> &'static str {
        match self {
            Tonemap::Hable => "hable",
            Tonemap::Mobius => "mobius",
            Tonemap::Reinhard => "reinhard",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RateMode {
    TargetSize { gb: f64 },
    Quality { crf: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SubtitleChoice {
    Embedded { index: u32 },
    External { path: String },
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodeParams {
    pub resolution: Resolution,
    pub rate_mode: RateMode,
    pub speed: Speed,
    pub hwaccel: bool,
    pub tonemap: Tonemap,
    pub audio_track_index: u32,
    pub audio_bitrate_k: u32,
    pub subtitle: SubtitleChoice,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleSpec {
    pub start_secs: f64,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullEstimate {
    pub size_bytes: u64,
    pub bitrate_k: u32,
    pub elapsed_secs: f64,
    pub suggested: Option<EncodeParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleResult {
    pub output_path: String,
    pub size_bytes: u64,
    pub avg_bitrate_k: u32,
    pub elapsed_secs: f64,
    pub full_estimate: FullEstimate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AuthKind {
    Password,
    Privatekey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_kind: AuthKind,
    pub private_key_path: Option<String>,
    pub remote_dir: String,
    pub url_prefix: String,
    pub access_key: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    Pending,
    Encoding,
    Encoded,
    Uploading,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub id: String,
    pub media: MediaInfo,
    pub params: EncodeParams,
    pub status: ItemStatus,
    pub encode_progress: f32,
    pub upload_progress: f32,
    pub result_url: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QueueMode {
    Pipeline,
    ParallelEncodeSerialUpload,
    ParallelAll,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSpace {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegPaths {
    pub ffmpeg: String,
    pub ffprobe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub language: String,
    pub default_resolution: Resolution,
    pub default_speed: Speed,
    pub default_tonemap: Tonemap,
    pub default_audio_bitrate_k: u32,
    pub default_queue_mode: QueueMode,
    pub active_profile_id: Option<String>,
    /// 样片压完后自动删除样片输出文件。兼容旧 settings.json：缺字段时按 default()=true 回填。
    #[serde(default)]
    pub auto_delete_sample: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            language: "zh".into(),
            default_resolution: Resolution::P1440,
            default_speed: Speed::Medium,
            default_tonemap: Tonemap::Hable,
            default_audio_bitrate_k: 192,
            default_queue_mode: QueueMode::Pipeline,
            active_profile_id: None,
            auto_delete_sample: true,
        }
    }
}

// ---- 事件载荷 ----

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub item_id: String,
    pub stage: String, // "encode" | "upload" | "analyze" | "download"
    pub percent: f32,
    pub speed: String,
    pub eta_secs: Option<f64>,
    // 编码详情（ffmpeg -progress 字段映射）
    pub frame: Option<u64>,
    pub total_frames: Option<u64>,
    pub fps: Option<f64>,
    pub bitrate: Option<String>,
    pub q: Option<f64>,
    pub out_time_secs: Option<f64>,
    pub total_secs: Option<f64>,
    pub cur_size: Option<u64>,
    // 上传详情
    pub transferred: Option<u64>,
    pub total_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub item_id: Option<String>,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEvent {
    pub percent: f32,
    pub message: String,
}

// ---- 历史记录 / 参数预设 ----

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub file_name: String,
    pub output_path: String,
    pub url: String,
    pub size_bytes: u64,
    pub created_at: String, // 由前端传入的本地时间字符串
    pub profile_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub params: EncodeParams,
}
