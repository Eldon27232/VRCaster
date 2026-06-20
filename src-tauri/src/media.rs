//! 媒体分析：ffprobe 解析 HDR/音轨/字幕轨/时长/分辨率。
//! 契约见 docs/contract.md §4。

use crate::types::{AudioTrack, MediaInfo, SampleSpec, SubtitleTrack};
use serde::Deserialize;
use tauri::AppHandle;

/// ffprobe -print_format json 的顶层结构（只取我们需要的字段）。
#[derive(Debug, Deserialize)]
struct ProbeOutput {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    format: Option<ProbeFormat>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    /// 全局流序号，与 -map 0:index 一致。
    index: u32,
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    color_transfer: Option<String>,
    channels: Option<u32>,
    #[serde(default)]
    tags: ProbeTags,
}

#[derive(Debug, Default, Deserialize)]
struct ProbeTags {
    language: Option<String>,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
    size: Option<String>,
}

/// 把 ffprobe 的分数帧率字符串（如 "24000/1001" 或 "30/1"）解析为 f64 fps。
fn parse_frame_rate(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some((num, den)) = s.split_once('/') {
        let num: f64 = num.trim().parse().ok()?;
        let den: f64 = den.trim().parse().ok()?;
        if den == 0.0 {
            return None;
        }
        Some(num / den)
    } else {
        s.parse().ok()
    }
}

/// 用 ffprobe 分析媒体文件，返回 MediaInfo。
#[tauri::command]
pub async fn analyze_media(app: AppHandle, path: String) -> Result<MediaInfo, String> {
    let ff = crate::ffmpeg::ensure(&app).await?;

    let output = run_ffprobe(&ff.ffprobe, &path).await?;

    let probe: ProbeOutput = serde_json::from_slice(&output)
        .map_err(|e| format!("解析 ffprobe JSON 失败: {e}"))?;

    // 视频流：取首条 codec_type == "video"。
    let video = probe
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"));

    let (width, height, fps, color_transfer) = match video {
        Some(v) => {
            let width = v.width.unwrap_or(0);
            let height = v.height.unwrap_or(0);
            // 优先 avg_frame_rate，退回 r_frame_rate。
            let fps = v
                .avg_frame_rate
                .as_deref()
                .and_then(parse_frame_rate)
                .filter(|f| *f > 0.0)
                .or_else(|| v.r_frame_rate.as_deref().and_then(parse_frame_rate))
                .unwrap_or(0.0);
            let color_transfer = v.color_transfer.clone();
            (width, height, fps, color_transfer)
        }
        None => (0, 0, 0.0, None),
    };

    let is_hdr = matches!(
        color_transfer.as_deref(),
        Some("smpte2084") | Some("arib-std-b67")
    );

    // 音轨。
    let audio_tracks: Vec<AudioTrack> = probe
        .streams
        .iter()
        .filter(|s| s.codec_type.as_deref() == Some("audio"))
        .map(|s| AudioTrack {
            index: s.index,
            codec: s.codec_name.clone().unwrap_or_default(),
            channels: s.channels.unwrap_or(0),
            language: s.tags.language.clone(),
            title: s.tags.title.clone(),
        })
        .collect();

    // 字幕轨。
    let subtitle_tracks: Vec<SubtitleTrack> = probe
        .streams
        .iter()
        .filter(|s| s.codec_type.as_deref() == Some("subtitle"))
        .map(|s| SubtitleTrack {
            index: s.index,
            codec: s.codec_name.clone().unwrap_or_default(),
            language: s.tags.language.clone(),
            title: s.tags.title.clone(),
        })
        .collect();

    let (duration_secs, size_bytes) = match &probe.format {
        Some(f) => {
            let duration_secs = f
                .duration
                .as_deref()
                .and_then(|d| d.trim().parse::<f64>().ok())
                .unwrap_or(0.0);
            let size_bytes = f
                .size
                .as_deref()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);
            (duration_secs, size_bytes)
        }
        None => (0.0, 0),
    };

    Ok(MediaInfo {
        path,
        duration_secs,
        width,
        height,
        fps,
        is_hdr,
        color_transfer,
        size_bytes,
        audio_tracks,
        subtitle_tracks,
    })
}

/// 运行 ffprobe 并返回其 stdout（JSON 字节）。Windows 隐藏黑窗。
async fn run_ffprobe(ffprobe: &str, path: &str) -> Result<Vec<u8>, String> {
    let mut cmd = tokio::process::Command::new(ffprobe);
    cmd.args([
        "-v",
        "quiet",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        path,
    ]);

    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("启动 ffprobe 失败: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "ffprobe 退出码 {:?}: {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    Ok(output.stdout)
}

/// 默认样片区间：正中间 2 分钟（已实现）。
#[tauri::command]
pub fn default_sample_spec(duration_secs: f64) -> SampleSpec {
    let dur = 120.0_f64.min(duration_secs.max(1.0));
    let start = (duration_secs / 2.0 - dur / 2.0).max(0.0);
    SampleSpec {
        start_secs: start,
        duration_secs: dur,
    }
}

/// 列出目录下的视频文件（一级，不递归），按文件名排序。供"拖入文件夹批量"使用。
#[tauri::command]
pub fn list_videos_in_dir(path: String) -> Result<Vec<String>, String> {
    const EXTS: &[&str] = &[
        "mkv", "mp4", "avi", "mov", "webm", "ts", "m4v", "flv", "wmv", "mpg", "mpeg", "m2ts",
    ];
    let mut out: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(&path).map_err(|e| format!("读取目录失败: {e}"))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if EXTS.contains(&ext.to_lowercase().as_str()) {
                    out.push(p.to_string_lossy().into_owned());
                }
            }
        }
    }
    out.sort();
    Ok(out)
}
