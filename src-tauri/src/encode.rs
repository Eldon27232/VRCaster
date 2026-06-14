//! 编码：构建 ffmpeg 命令（HDR→SDR tonemap / 缩放 / 字幕硬编）、执行、进度解析、样片。
//! 契约见 docs/contract.md §4、§5。

use crate::ffmpeg;
use crate::subtitle;
use crate::types::{
    EncodeParams, FfmpegPaths, FullEstimate, MediaInfo, ProgressEvent, RateMode, SampleResult,
    SampleSpec, SubtitleChoice,
};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 给 tokio::process::Command 套上 Windows 隐藏黑窗标志（其他平台 no-op）。
fn hide_window(cmd: &mut Command) {
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
}

/// 构建滤镜链：HDR→SDR tonemap + 缩放 + 可选字幕硬编。
///
/// `sub_path` 为工作目录内的相对 .ass 文件名（如 "subs.ass"），避免 Windows 冒号转义问题。
fn build_filter(params: &EncodeParams, media: &MediaInfo, sub_path: Option<&str>) -> String {
    let (w, h) = params.resolution.dims();
    let mut filter = if media.is_hdr {
        format!(
            "zscale=w={w}:h={h}:t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,\
             tonemap=tonemap={algo}:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p",
            w = w,
            h = h,
            algo = params.tonemap.as_str(),
        )
    } else {
        format!("scale={w}:{h}:flags=lanczos,format=yuv420p", w = w, h = h)
    };
    if let Some(sub) = sub_path {
        // 用相对文件名引用工作目录内的 .ass，子进程 current_dir 已设为该目录。
        filter.push_str(&format!(",subtitles={}", sub));
    }
    filter
}

/// 计算目标大小模式下的视频码率（kbps）。返回至少 1 的正值。
fn target_video_kbps(target_bytes: u64, duration_secs: f64, audio_bitrate_k: u32) -> u32 {
    if duration_secs <= 0.0 {
        return 1;
    }
    let total_k = (target_bytes as f64) * 8.0 / duration_secs / 1000.0;
    let video_k = total_k - audio_bitrate_k as f64;
    if video_k < 1.0 {
        1
    } else {
        video_k.round() as u32
    }
}

/// 构建完整 ffmpeg 参数列表（不含可执行名）。
///
/// `filter` 为已构建好的滤镜链；`duration_secs` 仅用于目标大小模式反算码率。
fn build_args(
    input: &str,
    out_path: &str,
    params: &EncodeParams,
    duration_secs: f64,
    filter: &str,
    sample: Option<&SampleSpec>,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // 覆盖输出、隐藏 banner，进度走 stdout。
    args.push("-y".into());
    args.push("-hide_banner".into());

    // GPU 硬解（探测失败 ffmpeg 自动回退软解）。
    if params.hwaccel {
        args.push("-hwaccel".into());
        args.push("auto".into());
    }

    // 样片裁剪：-ss/-t 放在 -i 前对解码更高效。
    if let Some(spec) = sample {
        args.push("-ss".into());
        args.push(format!("{}", spec.start_secs));
    }

    args.push("-i".into());
    args.push(input.into());

    if let Some(spec) = sample {
        args.push("-t".into());
        args.push(format!("{}", spec.duration_secs));
    }

    // 滤镜链。
    args.push("-vf".into());
    args.push(filter.into());

    // 轨道映射：视频 0:0 + 指定音轨。
    args.push("-map".into());
    args.push("0:0".into());
    args.push("-map".into());
    args.push(format!("0:{}", params.audio_track_index));

    // 锁死视频预设。
    args.push("-c:v".into());
    args.push("libx264".into());
    args.push("-profile:v".into());
    args.push("main".into());
    args.push("-pix_fmt".into());
    args.push("yuv420p".into());
    args.push("-preset".into());
    args.push(params.speed.preset().into());
    args.push("-x264-params".into());
    args.push("keyint=48:min-keyint=48:bframes=0:scenecut=0".into());
    args.push("-fps_mode".into());
    args.push("cfr".into());

    // 码率控制。
    match params.rate_mode {
        RateMode::TargetSize { gb } => {
            let target_bytes = (gb * 1024.0 * 1024.0 * 1024.0) as u64;
            let kbps = target_video_kbps(target_bytes, duration_secs, params.audio_bitrate_k);
            args.push("-b:v".into());
            args.push(format!("{}k", kbps));
            args.push("-maxrate".into());
            args.push(format!("{}k", (kbps as f64 * 1.2).round() as u32));
            args.push("-bufsize".into());
            args.push(format!("{}k", kbps.saturating_mul(2)));
        }
        RateMode::Quality { crf } => {
            args.push("-crf".into());
            args.push(format!("{}", crf));
        }
    }

    // 音频：AAC 立体声。
    args.push("-c:a".into());
    args.push("aac".into());
    args.push("-ac".into());
    args.push("2".into());
    args.push("-b:a".into());
    args.push(format!("{}k", params.audio_bitrate_k));

    // 容器收尾：faststart + 丢弃数据流。
    args.push("-movflags".into());
    args.push("+faststart".into());
    args.push("-dn".into());

    // 进度输出到 stdout（pipe），逐行 key=value。
    args.push("-progress".into());
    args.push("pipe:1".into());
    args.push("-nostats".into());

    args.push(out_path.into());
    args
}

/// 准备字幕：内嵌轨提取到工作目录 subs.ass 并返回相对名；外挂返回其转义后的绝对路径。
///
/// 返回 (用于滤镜的路径串, 子进程工作目录)。工作目录恒为 out_path 的父目录。
fn prepare_subtitle(
    media: &MediaInfo,
    params: &EncodeParams,
    out_path: &str,
    ffmpeg_exe: &str,
) -> Result<(Option<String>, PathBuf), String> {
    let work_dir = Path::new(out_path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    match &params.subtitle {
        SubtitleChoice::Embedded { index } => {
            let ass_name = "subs.ass";
            let ass_full = work_dir.join(ass_name);
            subtitle::extract_track(
                &media.path,
                *index,
                ass_full.to_string_lossy().as_ref(),
                ffmpeg_exe,
            )?;
            // 工作目录设为 work_dir 后，滤镜用相对名引用，规避 Windows 冒号转义。
            Ok((Some(ass_name.to_string()), work_dir))
        }
        SubtitleChoice::External { path } => {
            // 外挂字幕：转义为可被 subtitles 滤镜接受的绝对路径。
            Ok((Some(escape_subtitle_path(path)), work_dir))
        }
        SubtitleChoice::None => Ok((None, work_dir)),
    }
}

/// 转义 subtitles 滤镜里的路径：Windows 盘符冒号、反斜杠、滤镜分隔符。
fn escape_subtitle_path(path: &str) -> String {
    // ffmpeg subtitles 滤镜值需要转义 `\` `:` 以及外层 `'`。
    // 形如 C:\a\b.ass -> C\:\\a\\b.ass
    let mut out = String::with_capacity(path.len() + 8);
    for ch in path.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ':' => out.push_str("\\:"),
            '\'' => out.push_str("\\'"),
            _ => out.push(ch),
        }
    }
    out
}

/// 解析 -progress 一行 `key=value`，更新累积状态。
struct ProgressState {
    out_time_secs: f64,
    speed: String,
}

impl ProgressState {
    fn new() -> Self {
        ProgressState {
            out_time_secs: 0.0,
            speed: String::new(),
        }
    }

    /// 处理一行，返回 true 表示遇到 `progress=`（一组进度刷新的结束标记），此时应上报一次。
    fn feed(&mut self, line: &str) -> bool {
        let line = line.trim();
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        match key {
            "out_time_us" | "out_time_ms" => {
                // out_time_us 是微秒；旧版 out_time_ms 实际也是微秒（ffmpeg 历史命名坑）。
                if let Ok(us) = value.trim().parse::<i64>() {
                    if us >= 0 {
                        self.out_time_secs = us as f64 / 1_000_000.0;
                    }
                }
            }
            "out_time" => {
                // 形如 HH:MM:SS.micro
                if let Some(secs) = parse_hhmmss(value.trim()) {
                    self.out_time_secs = secs;
                }
            }
            "speed" => {
                // 形如 "1.23x"
                self.speed = value.trim().to_string();
            }
            "progress" => {
                // value 为 "continue" 或 "end"，两者都代表本组进度刷新结束。
                return true;
            }
            _ => {}
        }
        false
    }
}

/// 解析 HH:MM:SS.micro 为秒。
fn parse_hhmmss(s: &str) -> Option<f64> {
    let mut parts = s.split(':');
    let h: f64 = parts.next()?.parse().ok()?;
    let m: f64 = parts.next()?.parse().ok()?;
    let sec: f64 = parts.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + sec)
}

/// 从 speed 串（"1.23x"）解析倍率，用于 ETA。
fn parse_speed(speed: &str) -> Option<f64> {
    let trimmed = speed.trim().trim_end_matches('x').trim();
    let v: f64 = trimmed.parse().ok()?;
    if v > 0.0 {
        Some(v)
    } else {
        None
    }
}

/// 运行一次 ffmpeg 编码，解析 -progress 并上报进度。
///
/// `total_duration` 为进度百分比的分母（全片用 media.duration_secs，样片用 spec.duration_secs）。
async fn spawn_and_track(
    app: Option<&AppHandle>,
    item_id: &str,
    ffmpeg_exe: &str,
    args: &[String],
    work_dir: &Path,
    total_duration: f64,
) -> Result<(), String> {
    let mut cmd = Command::new(ffmpeg_exe);
    cmd.args(args)
        .current_dir(work_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_window(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 ffmpeg 失败: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法获取 ffmpeg stdout".to_string())?;

    // 单独把 stderr 读干净，避免管道写满导致 ffmpeg 阻塞；同时留作错误诊断。
    let stderr = child.stderr.take();
    let stderr_handle = tokio::spawn(async move {
        let mut buf = String::new();
        if let Some(stderr) = stderr {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        // 仅保留末尾若干内容用于报错，避免无限增长。
                        buf.push_str(&line);
                        if buf.len() > 8192 {
                            let cut = buf.len() - 8192;
                            buf.drain(0..cut);
                        }
                    }
                }
            }
        }
        buf
    });

    let mut reader = BufReader::new(stdout);
    let mut state = ProgressState::new();
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("读取 ffmpeg 进度失败: {}", e))?;
        if n == 0 {
            break;
        }
        let group_end = state.feed(&line);
        if group_end {
            if let Some(app) = app {
                let percent = if total_duration > 0.0 {
                    ((state.out_time_secs / total_duration) * 100.0).clamp(0.0, 100.0) as f32
                } else {
                    0.0
                };
                let eta_secs = parse_speed(&state.speed).and_then(|sp| {
                    let remain = total_duration - state.out_time_secs;
                    if remain > 0.0 {
                        Some(remain / sp)
                    } else {
                        Some(0.0)
                    }
                });
                let _ = app.emit(
                    "progress",
                    ProgressEvent {
                        item_id: item_id.to_string(),
                        stage: "encode".into(),
                        percent,
                        speed: state.speed.clone(),
                        eta_secs,
                    },
                );
            }
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("等待 ffmpeg 退出失败: {}", e))?;

    let stderr_tail = stderr_handle.await.unwrap_or_default();

    if !status.success() {
        let code = status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "信号终止".into());
        return Err(format!(
            "ffmpeg 退出码 {}：\n{}",
            code,
            stderr_tail.trim_end()
        ));
    }

    // 收尾推一个 100% 进度。
    if let Some(app) = app {
        let _ = app.emit(
            "progress",
            ProgressEvent {
                item_id: item_id.to_string(),
                stage: "encode".into(),
                percent: 100.0,
                speed: state.speed.clone(),
                eta_secs: Some(0.0),
            },
        );
    }

    Ok(())
}

/// 压制全片，进度通过 "progress" 事件上报，返回成品路径（内部 fn，命令为薄封装）。
pub async fn run_encode(
    app: &AppHandle,
    item_id: &str,
    media: &MediaInfo,
    params: &EncodeParams,
    ff: &FfmpegPaths,
    out_path: &str,
) -> Result<(), String> {
    let (sub_path, work_dir) = prepare_subtitle(media, params, out_path, &ff.ffmpeg)?;
    let filter = build_filter(params, media, sub_path.as_deref());
    let args = build_args(
        &media.path,
        out_path,
        params,
        media.duration_secs,
        &filter,
        None,
    );
    spawn_and_track(
        Some(app),
        item_id,
        &ff.ffmpeg,
        &args,
        &work_dir,
        media.duration_secs,
    )
    .await
}

/// 由 media 路径与 params 推导成品输出路径：媒体同目录下 `<stem>_vrc.mp4`。
fn derive_out_path(media: &MediaInfo) -> Result<String, String> {
    let src = Path::new(&media.path);
    let stem = src
        .file_stem()
        .map(|s| sanitize_stem(&s.to_string_lossy()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "output".to_string());
    let dir = src
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(crate::config::data_dir);
    let out = dir.join(format!("{}_vrc.mp4", stem));
    Ok(out.to_string_lossy().to_string())
}

/// 把文件名 stem 中可能引起命令/路径问题的字符替换为下划线，并保证非空英文友好。
fn sanitize_stem(stem: &str) -> String {
    let mut out: String = stem
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();
    out = out.trim().trim_end_matches('.').to_string();
    out
}

/// #[tauri::command] 全片压制：ensure ffmpeg → 定 out_path → run_encode → 返回成品路径。
#[tauri::command]
pub async fn start_encode(
    app: AppHandle,
    item_id: String,
    media: MediaInfo,
    params: EncodeParams,
) -> Result<String, String> {
    let ff = ffmpeg::ensure(&app).await?;
    let out_path = derive_out_path(&media)?;

    // 确保输出目录存在。
    if let Some(parent) = Path::new(&out_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
        }
    }

    run_encode(&app, &item_id, &media, &params, &ff, &out_path).await?;
    Ok(out_path)
}

/// 反算：给定样片实测平均码率，构造目标大小模式下建议参数（按目标大小反算 video 码率不直接改 params，
/// 因为 ABR 由 rate_mode 表达；这里仅在目标大小模式回填同一目标，保持 params 语义不变）。
fn build_suggested(params: &EncodeParams) -> Option<EncodeParams> {
    match params.rate_mode {
        // 目标大小模式：建议沿用当前目标（前端可据 full_estimate 再调）。
        RateMode::TargetSize { .. } => Some(params.clone()),
        // 质量模式：不提供反算建议。
        RateMode::Quality { .. } => None,
    }
}

/// #[tauri::command] 压制样片并外推全片：用 -ss/-t 截取，同滤镜同参数。
#[tauri::command]
pub async fn encode_sample(
    app: AppHandle,
    media: MediaInfo,
    params: EncodeParams,
    spec: SampleSpec,
) -> Result<SampleResult, String> {
    let ff = ffmpeg::ensure(&app).await?;

    // 样片输出放到 data_dir 下临时名，避免污染媒体目录。
    let data_dir = crate::config::data_dir();
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
    let sample_out = data_dir.join("vrc_sample.mp4");
    let sample_out_str = sample_out.to_string_lossy().to_string();

    let (sub_path, work_dir) = prepare_subtitle(&media, &params, &sample_out_str, &ff.ffmpeg)?;
    let filter = build_filter(&params, &media, sub_path.as_deref());
    // 样片码率控制用样片时长作为目标大小反算分母（保证与全片同等“每秒码率”行为下，
    // 目标大小模式 ffmpeg 用的是码率而非总大小，故分母用全片时长才能复现全片码率）。
    let args = build_args(
        &media.path,
        &sample_out_str,
        &params,
        media.duration_secs,
        &filter,
        Some(&spec),
    );

    let started = std::time::Instant::now();
    // 样片进度也上报（item_id 用固定标识，前端可忽略）。
    spawn_and_track(
        Some(&app),
        "__sample__",
        &ff.ffmpeg,
        &args,
        &work_dir,
        spec.duration_secs,
    )
    .await?;
    let elapsed_secs = started.elapsed().as_secs_f64();

    let size_bytes = std::fs::metadata(&sample_out)
        .map_err(|e| format!("读取样片大小失败: {}", e))?
        .len();

    let sample_dur = if spec.duration_secs > 0.0 {
        spec.duration_secs
    } else {
        1.0
    };
    let avg_bitrate_k = ((size_bytes as f64) * 8.0 / sample_dur / 1000.0).round() as u32;

    // 全片外推。
    let full_size = ((avg_bitrate_k as f64) * 1000.0 / 8.0 * media.duration_secs).round() as u64;
    let full_elapsed = elapsed_secs * media.duration_secs / sample_dur;

    let full_estimate = FullEstimate {
        size_bytes: full_size,
        bitrate_k: avg_bitrate_k,
        elapsed_secs: full_elapsed,
        suggested: build_suggested(&params),
    };

    Ok(SampleResult {
        output_path: sample_out_str,
        size_bytes,
        avg_bitrate_k,
        elapsed_secs,
        full_estimate,
    })
}
