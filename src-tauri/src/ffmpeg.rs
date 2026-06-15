//! 首次检测/下载 ffmpeg 与 ffprobe 到 app data 目录。
//! 契约见 docs/contract.md §4。

use crate::types::{DownloadEvent, FfmpegPaths};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use tauri::{AppHandle, Emitter};

const DOWNLOAD_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

/// 确保 ffmpeg/ffprobe 可用：先查 app data，否则下载 gyan.dev 构建并解压，进度走 "ffmpeg-download" 事件。
pub async fn ensure(app: &AppHandle) -> Result<FfmpegPaths, String> {
    let ffmpeg_dir = crate::config::data_dir().join("ffmpeg");
    let ffmpeg_exe = ffmpeg_dir.join("ffmpeg.exe");
    let ffprobe_exe = ffmpeg_dir.join("ffprobe.exe");

    // 1) 已下载到 app data → 直接返回。
    if ffmpeg_exe.is_file() && ffprobe_exe.is_file() {
        return Ok(FfmpegPaths {
            ffmpeg: ffmpeg_exe.to_string_lossy().into_owned(),
            ffprobe: ffprobe_exe.to_string_lossy().into_owned(),
        });
    }

    // 2) 系统 PATH 已装 ffmpeg/ffprobe → 直接用，免下载（国内下载源不可靠）。
    if let (Some(fm), Some(fp)) = (find_in_path("ffmpeg"), find_in_path("ffprobe")) {
        return Ok(FfmpegPaths {
            ffmpeg: fm,
            ffprobe: fp,
        });
    }

    fs::create_dir_all(&ffmpeg_dir).map_err(|e| format!("创建 ffmpeg 目录失败: {e}"))?;

    // 2) 流式下载 zip 到临时文件，按 content-length 累计进度。
    let emit = |percent: f32, message: &str| {
        let _ = app.emit(
            "ffmpeg-download",
            DownloadEvent {
                percent,
                message: message.to_string(),
            },
        );
    };

    emit(0.0, "开始下载 ffmpeg…");

    let resp = reqwest::get(DOWNLOAD_URL)
        .await
        .map_err(|e| format!("请求 ffmpeg 下载失败: {e}"))?;
    let resp = resp
        .error_for_status()
        .map_err(|e| format!("ffmpeg 下载响应错误: {e}"))?;

    let total = resp.content_length();

    let tmp_zip = ffmpeg_dir.join("ffmpeg-download.zip");
    let mut file = fs::File::create(&tmp_zip).map_err(|e| format!("创建临时文件失败: {e}"))?;

    let mut downloaded: u64 = 0;
    let mut last_emit_pct: f32 = -1.0;
    let mut resp = resp;
    loop {
        let chunk = resp
            .chunk()
            .await
            .map_err(|e| format!("下载 ffmpeg 数据出错: {e}"))?;
        let chunk = match chunk {
            Some(c) => c,
            None => break,
        };
        file.write_all(&chunk)
            .map_err(|e| format!("写入临时文件失败: {e}"))?;
        downloaded += chunk.len() as u64;

        if let Some(total) = total {
            if total > 0 {
                let pct = (downloaded as f32 / total as f32) * 100.0;
                // 限频：每变化 ≥1% 才 emit，避免事件风暴。
                if pct - last_emit_pct >= 1.0 {
                    last_emit_pct = pct;
                    emit(
                        pct,
                        &format!(
                            "下载中 {:.1}/{:.1} MB",
                            downloaded as f64 / 1_048_576.0,
                            total as f64 / 1_048_576.0
                        ),
                    );
                }
            }
        } else {
            emit(0.0, &format!("下载中 {:.1} MB", downloaded as f64 / 1_048_576.0));
        }
    }
    file.flush().map_err(|e| format!("刷新临时文件失败: {e}"))?;
    drop(file);

    emit(100.0, "下载完成，正在解压…");

    // 3) 解压，从 bin/ 取出 ffmpeg.exe 与 ffprobe.exe。
    let extract_res = extract_executables(&tmp_zip, &ffmpeg_dir);
    // 不论解压成败都清理临时 zip。
    let _ = fs::remove_file(&tmp_zip);
    extract_res?;

    if !ffmpeg_exe.is_file() || !ffprobe_exe.is_file() {
        return Err("解压完成但未找到 ffmpeg.exe / ffprobe.exe".into());
    }

    emit(100.0, "ffmpeg 准备就绪");

    Ok(FfmpegPaths {
        ffmpeg: ffmpeg_exe.to_string_lossy().into_owned(),
        ffprobe: ffprobe_exe.to_string_lossy().into_owned(),
    })
}

/// 在系统 PATH 中查找可执行文件（Windows `where`），返回首个绝对路径。
fn find_in_path(name: &str) -> Option<String> {
    use std::process::Command;
    let mut cmd = Command::new("where");
    cmd.arg(name);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .map(|s| s.to_string())
}

/// 从 zip 中提取 bin/ffmpeg.exe 与 bin/ffprobe.exe 到目标目录（扁平化，去掉路径前缀）。
fn extract_executables(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let zip_file = fs::File::open(zip_path).map_err(|e| format!("打开 zip 失败: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(zip_file).map_err(|e| format!("解析 zip 失败: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("读取 zip 条目失败: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        // gyan.dev 构建目录形如 ffmpeg-x.y-essentials_build/bin/ffmpeg.exe
        let name = entry.name().replace('\\', "/");
        let file_name = match name.rsplit('/').next() {
            Some(f) => f,
            None => continue,
        };
        let target = match file_name {
            "ffmpeg.exe" if name.contains("/bin/") || name.ends_with("/bin/ffmpeg.exe") => {
                dest_dir.join("ffmpeg.exe")
            }
            "ffprobe.exe" if name.contains("/bin/") || name.ends_with("/bin/ffprobe.exe") => {
                dest_dir.join("ffprobe.exe")
            }
            _ => continue,
        };

        let mut out =
            fs::File::create(&target).map_err(|e| format!("创建 {file_name} 失败: {e}"))?;
        // 分块拷贝，避免一次性载入大文件。
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = entry
                .read(&mut buf)
                .map_err(|e| format!("读取 {file_name} 失败: {e}"))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])
                .map_err(|e| format!("写入 {file_name} 失败: {e}"))?;
        }
    }

    Ok(())
}

/// 确保 ffmpeg/ffprobe 可用；首次调用会下载并解压。
#[tauri::command]
pub async fn ensure_ffmpeg(app: AppHandle) -> Result<FfmpegPaths, String> {
    ensure(&app).await
}
