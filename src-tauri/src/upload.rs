//! SFTP 上传 + 断点续传（russh + russh-sftp，纯 Rust，免 openssl）。
//! 契约见 docs/contract.md §4、§5。

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use russh::client::{self, Config, Handle, Handler};
use russh::keys::ssh_key::PublicKey;
use russh::keys::{decode_secret_key, load_secret_key, PrivateKeyWithHashAlg};
use russh::ChannelMsg;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{OpenFlags, StatusCode};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::types::{AuthKind, ProgressEvent, RemoteSpace, ServerProfile};

/// 单次读写的分块大小（256 KiB，与 russh-sftp 默认 max_packet_len 对齐）。
const CHUNK: usize = 256 * 1024;

/// 最小 russh 客户端 handler：接受任意服务器公钥（信任由调用方负责）。
struct Client;

impl Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        // 桌面工具场景：不做 known_hosts 校验，接受任意服务器公钥。
        Ok(true)
    }
}

/// 建立 SSH 连接并完成认证，返回已认证的会话 handle。
async fn connect_and_auth(
    profile: &ServerProfile,
    secret: Option<&str>,
) -> Result<Handle<Client>, String> {
    let config = Arc::new(Config::default());
    let mut handle = client::connect(config, (profile.host.as_str(), profile.port), Client)
        .await
        .map_err(|e| format!("SSH 连接 {}:{} 失败: {}", profile.host, profile.port, e))?;

    let authed = match profile.auth_kind {
        AuthKind::Password => {
            let password = secret.ok_or_else(|| "密码认证缺少凭据（keyring 未找到）".to_string())?;
            handle
                .authenticate_password(profile.username.as_str(), password)
                .await
                .map_err(|e| format!("密码认证失败: {}", e))?
        }
        AuthKind::Privatekey => {
            let key_path = profile
                .private_key_path
                .as_deref()
                .ok_or_else(|| "私钥认证缺少 private_key_path".to_string())?;
            // secret 为私钥口令（无口令则为 None）。
            let key = load_secret_key(key_path, secret)
                .or_else(|e| {
                    // 兜底：路径可能直接是私钥文本内容。
                    decode_secret_key(key_path, secret).map_err(|_| e)
                })
                .map_err(|e| format!("加载私钥 {} 失败: {}", key_path, e))?;
            let key = PrivateKeyWithHashAlg::new(Arc::new(key), None);
            handle
                .authenticate_publickey(profile.username.as_str(), key)
                .await
                .map_err(|e| format!("私钥认证失败: {}", e))?
        }
    };

    if !authed.success() {
        return Err(format!("认证被服务器拒绝（用户 {}）", profile.username));
    }
    Ok(handle)
}

/// 在已认证会话上开启一个 SFTP 子系统会话。
async fn open_sftp(handle: &Handle<Client>) -> Result<SftpSession, String> {
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("打开 SSH 通道失败: {}", e))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| format!("请求 sftp 子系统失败: {}", e))?;
    SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("初始化 SFTP 会话失败: {}", e))
}

/// 查询远程文件大小；不存在返回 None。
async fn remote_size(sftp: &SftpSession, remote_path: &str) -> Result<Option<u64>, String> {
    match sftp.metadata(remote_path).await {
        Ok(attrs) => Ok(Some(attrs.size.unwrap_or(0))),
        Err(russh_sftp::client::error::Error::Status(s))
            if s.status_code == StatusCode::NoSuchFile =>
        {
            Ok(None)
        }
        Err(e) => Err(format!("查询远程文件 {} 失败: {}", remote_path, e)),
    }
}

/// 取本地文件名（用于远程路径与 URL）。
fn file_name(local_path: &str) -> Result<String, String> {
    Path::new(local_path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("无法从 {} 解析文件名", local_path))
}

/// 拼接远程目录与文件名（统一用正斜杠，远程为 POSIX 路径）。
fn join_remote(remote_dir: &str, filename: &str) -> String {
    format!("{}/{}", remote_dir.trim_end_matches('/'), filename)
}

/// 上传本地文件到 profile 指定服务器，断点续传，进度通过 "progress" 事件上报，返回最终 URL。
pub async fn run_upload(
    app: &AppHandle,
    item_id: &str,
    local_path: &str,
    profile: &ServerProfile,
    secret: Option<&str>,
) -> Result<String, String> {
    let filename = file_name(local_path)?;
    let remote_path = join_remote(&profile.remote_dir, &filename);

    // 本地总大小。
    let total = tokio::fs::metadata(local_path)
        .await
        .map_err(|e| format!("读取本地文件 {} 失败: {}", local_path, e))?
        .len();

    let handle = connect_and_auth(profile, secret).await?;
    let sftp = open_sftp(&handle).await?;

    let emit = |sent: u64, speed: String, eta: Option<f64>| {
        let percent = if total == 0 {
            100.0
        } else {
            (sent as f64 / total as f64 * 100.0) as f32
        };
        let _ = app.emit(
            "progress",
            ProgressEvent {
                item_id: item_id.to_string(),
                stage: "upload".into(),
                percent,
                speed,
                eta_secs: eta,
                transferred: Some(sent),
                total_size: Some(total),
                ..Default::default()
            },
        );
    };

    // 空文件：确保远程存在一个空文件即可。
    if total == 0 {
        let _ = sftp
            .open_with_flags(
                remote_path.as_str(),
                OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::TRUNCATE,
            )
            .await
            .map_err(|e| format!("创建远程空文件失败: {}", e))?;
        emit(0, "0 B/s".into(), Some(0.0));
        return Ok(build_url(profile, &filename));
    }

    let started = Instant::now();
    let mut last_emit = Instant::now();

    // 续传循环：每轮以远程当前大小为起点写入，直到远程大小==本地大小。
    // 多轮可在网络中断后由上层重试时自然续上；单轮内一次性写完。
    loop {
        let off = remote_size(&sftp, &remote_path).await?.unwrap_or(0);
        if off >= total {
            break; // 已传完（或远程被外部写大，按完成处理）。
        }

        // 本地从断点读起。
        let mut local = tokio::fs::File::open(local_path)
            .await
            .map_err(|e| format!("打开本地文件失败: {}", e))?;
        local
            .seek(std::io::SeekFrom::Start(off))
            .await
            .map_err(|e| format!("本地 seek 到 {} 失败: {}", off, e))?;

        // 远程以写模式打开（存在则不截断，定位到 off 续写；不存在则创建）。
        let flags = OpenFlags::WRITE | OpenFlags::CREATE;
        let mut remote = sftp
            .open_with_flags(remote_path.as_str(), flags)
            .await
            .map_err(|e| format!("打开远程文件失败: {}", e))?;
        remote
            .seek(std::io::SeekFrom::Start(off))
            .await
            .map_err(|e| format!("远程 seek 到 {} 失败: {}", off, e))?;

        let mut sent = off;
        let mut buf = vec![0u8; CHUNK];
        emit(sent, "0 B/s".into(), None);

        loop {
            let n = local
                .read(&mut buf)
                .await
                .map_err(|e| format!("读取本地数据失败: {}", e))?;
            if n == 0 {
                break;
            }
            remote
                .write_all(&buf[..n])
                .await
                .map_err(|e| format!("写入远程数据失败: {}", e))?;
            sent += n as u64;

            // 限频上报进度（约每 500ms）。
            if last_emit.elapsed().as_millis() >= 500 {
                let elapsed = started.elapsed().as_secs_f64().max(0.001);
                let bps = (sent.saturating_sub(off)) as f64 / elapsed;
                let remaining = total.saturating_sub(sent) as f64;
                let eta = if bps > 0.0 { Some(remaining / bps) } else { None };
                emit(sent, fmt_speed(bps), eta);
                last_emit = Instant::now();
            }
        }

        // 确保所有写请求落盘并暴露错误，再关闭句柄。
        remote
            .flush()
            .await
            .map_err(|e| format!("flush 远程文件失败: {}", e))?;
        remote
            .shutdown()
            .await
            .map_err(|e| format!("关闭远程文件失败: {}", e))?;
    }

    let elapsed = started.elapsed().as_secs_f64().max(0.001);
    let bps = total as f64 / elapsed;
    emit(total, fmt_speed(bps), Some(0.0));

    Ok(build_url(profile, &filename))
}

/// 构造带 key 鉴权的最终 URL：{url_prefix}/{filename}?key={access_key}
fn build_url(profile: &ServerProfile, filename: &str) -> String {
    format!(
        "{}/{}?key={}",
        profile.url_prefix.trim_end_matches('/'),
        filename,
        profile.access_key
    )
}

/// 人类可读速率。
fn fmt_speed(bps: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    if bps >= MB {
        format!("{:.1} MB/s", bps / MB)
    } else if bps >= KB {
        format!("{:.1} KB/s", bps / KB)
    } else {
        format!("{:.0} B/s", bps)
    }
}

/// 通过 SSH 执行命令并收集 stdout（含退出码）。
async fn ssh_exec(handle: &Handle<Client>, command: &str) -> Result<(String, u32), String> {
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("打开 SSH 通道失败: {}", e))?;
    channel
        .exec(true, command)
        .await
        .map_err(|e| format!("执行命令失败: {}", e))?;

    let mut stdout = Vec::new();
    let mut code: u32 = 0;
    while let Some(msg) = channel.wait().await {
        match msg {
            ChannelMsg::Data { ref data } => stdout.extend_from_slice(data),
            ChannelMsg::ExitStatus { exit_status } => code = exit_status,
            ChannelMsg::Eof | ChannelMsg::Close => {}
            _ => {}
        }
    }
    Ok((String::from_utf8_lossy(&stdout).into_owned(), code))
}

/// 解析 `df -k <dir>` 输出，返回（可用字节, 总字节）。
fn parse_df_k(out: &str) -> Option<(u64, u64)> {
    // 典型输出：
    // Filesystem     1K-blocks      Used Available Use% Mounted on
    // /dev/sda1       41251136  12345678  26905458  32% /
    // 取最后一行数据行（兼容 Filesystem 过长换行的情况，合并所有数字字段后定位）。
    let data_line = out
        .lines()
        .skip(1)
        .find(|l| l.split_whitespace().filter(|t| t.chars().all(|c| c.is_ascii_digit())).count() >= 3)?;
    let nums: Vec<u64> = data_line
        .split_whitespace()
        .filter_map(|t| t.parse::<u64>().ok())
        .collect();
    // 期望顺序：1K-blocks(total) Used Available。Available 是 nums 中第三个纯数字。
    if nums.len() >= 3 {
        let total = nums[0] * 1024;
        let available = nums[2] * 1024;
        Some((available, total))
    } else {
        None
    }
}

// ---- Tauri 命令（内部 fn 的薄封装）----

/// 上传本地文件到 profile 指定服务器，进度通过 "progress" 事件上报，返回最终 URL。
#[tauri::command]
pub async fn start_upload(
    app: AppHandle,
    item_id: String,
    local_path: String,
    profile_id: String,
) -> Result<String, String> {
    let profile = crate::config::get_profile(&profile_id)?;
    let secret = crate::config::get_secret(&profile_id)?;
    run_upload(&app, &item_id, &local_path, &profile, secret.as_deref()).await
}

/// 查询远程磁盘空间（上传前预检）。
#[tauri::command]
pub async fn check_remote_space(profile_id: String) -> Result<RemoteSpace, String> {
    let profile = crate::config::get_profile(&profile_id)?;
    let secret = crate::config::get_secret(&profile_id)?;

    let handle = connect_and_auth(&profile, secret.as_deref()).await?;
    // df -k 输出单位为 1K 块，便于解析（POSIX 通用）。
    let cmd = format!("df -k {}", shell_quote(&profile.remote_dir));
    let (out, _code) = ssh_exec(&handle, &cmd).await?;
    let (free_bytes, total_bytes) =
        parse_df_k(&out).ok_or_else(|| format!("无法解析 df 输出:\n{}", out))?;
    Ok(RemoteSpace {
        free_bytes,
        total_bytes,
    })
}

/// 简单的 POSIX 单引号转义，防止远程目录含空格/特殊字符。
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
