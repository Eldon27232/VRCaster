//! 字幕：内嵌轨提取、内嵌字体提取、系统字体 fallback。供 encode 模块调用（非命令）。
//! 契约见 docs/contract.md §4、§5。

use std::fs;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Windows 下隐藏子进程黑窗（CREATE_NO_WINDOW）。非 Windows 平台为 no-op。
#[cfg(windows)]
fn hide_window(cmd: &mut Command) {
    cmd.creation_flags(0x08000000);
}
#[cfg(not(windows))]
fn hide_window(_cmd: &mut Command) {}

/// 提取指定字幕轨到 .ass 文件（相对工作目录的简单文件名，便于 subtitles 滤镜引用）。
/// 等价命令：ffmpeg -y -i <input> -map 0:<index> -c copy <out>
pub fn extract_track(input: &str, index: u32, out: &str, ffmpeg: &str) -> Result<(), String> {
    let mut cmd = Command::new(ffmpeg);
    cmd.arg("-y")
        .arg("-i")
        .arg(input)
        .arg("-map")
        .arg(format!("0:{index}"))
        .arg("-c")
        .arg("copy")
        .arg(out);
    hide_window(&mut cmd);

    let output = cmd
        .output()
        .map_err(|e| format!("启动 ffmpeg 提取字幕轨失败: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "ffmpeg 提取字幕轨 0:{index} 失败 (exit {}): {}",
            output
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".into()),
            stderr.trim()
        ));
    }
    Ok(())
}

/// 提取内嵌字体附件到目录，供 libass 使用。
/// 等价命令：在 <dir> 内执行 ffmpeg -y -dump_attachment:t "" -i <input>
/// 该命令在无附件时返回非零退出码——视为成功，缺失字体由系统 fontconfig fallback。
pub fn extract_fonts(input: &str, dir: &str, ffmpeg: &str) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("创建字体目录 {dir} 失败: {e}"))?;

    let mut cmd = Command::new(ffmpeg);
    cmd.current_dir(dir)
        .arg("-y")
        .arg("-dump_attachment:t")
        .arg("")
        .arg("-i")
        .arg(input);
    hide_window(&mut cmd);

    // 无附件时 ffmpeg 以非零码退出（"At least one output file must be specified"），
    // 这是预期行为；只要进程能启动就视为成功，字体缺失靠系统 fontconfig fallback。
    match cmd.output() {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("启动 ffmpeg 提取字体失败: {e}")),
    }
}
