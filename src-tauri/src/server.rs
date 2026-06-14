//! nginx 配置生成 + SSH 一键部署。
//! 契约见 docs/contract.md §4、§5。

use std::sync::Arc;

use russh::client::{self, Handle, Handler};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};
use russh::ChannelMsg;

use crate::types::{AuthKind, ServerProfile};

/// nginx 配置写入路径（覆盖发行版默认站点）。
const REMOTE_CONF_PATH: &str = "/etc/nginx/sites-enabled/default";

/// 一键部署：SSH 装 nginx → 写带 key 鉴权配置 → nginx -t → reload，返回部署日志。
#[tauri::command]
pub async fn deploy_nginx(profile_id: String) -> Result<String, String> {
    let profile = crate::config::get_profile(&profile_id)?;
    let secret = crate::config::get_secret(&profile_id)?;

    // 复用与 upload 相同的 russh 连接方式：建立认证后的会话句柄。
    let handle = connect(&profile, secret.as_deref()).await?;

    let conf = gen_nginx_conf(&profile);
    // heredoc 写文件：用引号包住 EOF 标记，禁止变量展开，保证配置内容原样落盘。
    let write_conf_cmd = format!(
        "cat > {path} <<'VRC_NGINX_EOF'\n{conf}\nVRC_NGINX_EOF",
        path = REMOTE_CONF_PATH,
        conf = conf,
    );

    // 按契约顺序执行；每步记录命令 + 退出码 + 末尾输出。
    let steps: [(&str, String); 7] = [
        ("apt 更新软件源", "export DEBIAN_FRONTEND=noninteractive; apt-get update -qq".into()),
        ("安装 nginx", "export DEBIAN_FRONTEND=noninteractive; apt-get install -y -qq nginx".into()),
        ("写入 nginx 配置", write_conf_cmd),
        ("校验 nginx 配置", "nginx -t".into()),
        ("开机自启并启动 nginx", "systemctl enable --now nginx".into()),
        ("重载 nginx 配置", "systemctl reload nginx".into()),
        ("查询 nginx 状态", "systemctl is-active nginx".into()),
    ];

    let mut log = String::new();
    let mut failed = false;
    for (title, cmd) in steps {
        // 前一步失败后不再继续执行，避免在坏配置上 reload。
        if failed {
            log.push_str(&format!("\n[跳过] {title}（前序步骤失败）\n"));
            continue;
        }
        let out = exec(&handle, &cmd).await?;
        log.push_str(&format_step(title, &cmd, &out));
        if out.exit_code != 0 {
            failed = true;
        }
    }

    // 主动关闭连接（忽略关闭错误，日志已收集完毕）。
    let _ = handle
        .disconnect(russh::Disconnect::ByApplication, "", "en")
        .await;

    if failed {
        Err(log)
    } else {
        Ok(log)
    }
}

/// 生成带 `?key=` 鉴权的 nginx server 配置块（map $arg_key + location ~ \.mp4$）。
pub fn gen_nginx_conf(profile: &ServerProfile) -> String {
    // access_key 进入 map 的字符串字面量，转义双引号/反斜杠避免配置语法被破坏。
    let key = escape_nginx_string(&profile.access_key);
    // root 目录同样转义，目录名通常无特殊字符，但保持稳健。
    let root = escape_nginx_string(&profile.remote_dir);
    format!(
        "map $arg_key $vrc_key_ok {{\n    \
default 0;\n    \
\"{key}\" 1;\n\
}}\n\
\n\
server {{\n    \
listen 80 default_server;\n    \
listen [::]:80 default_server;\n    \
root {root};\n    \
server_name _;\n\
\n    \
location ~ \\.mp4$ {{\n        \
if ($vrc_key_ok = 0) {{\n            \
return 403;\n        \
}}\n    \
}}\n\
\n    \
location / {{\n        \
try_files $uri $uri/ =404;\n    \
}}\n\
}}\n",
        key = key,
        root = root,
    )
}

/// 转义进入 nginx 双引号字符串的内容：先反斜杠后双引号。
fn escape_nginx_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

// ---------------------------------------------------------------------------
// SSH exec 辅助（自包含，与 upload 复用同一 russh 连接/认证方式）
// ---------------------------------------------------------------------------

/// 单条命令执行结果：合并 stdout/stderr，记录退出码。
struct ExecOutput {
    exit_code: i32,
    output: String,
}

/// russh 客户端 Handler：仅做连接，信任服务器主机密钥（首次部署场景，无已知 hosts）。
struct SshClient;

impl Handler for SshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // 接受服务器密钥：部署目标由用户在 profile 中显式指定。
        Ok(true)
    }
}

/// 建立并认证 SSH 连接，返回会话句柄。
async fn connect(
    profile: &ServerProfile,
    secret: Option<&str>,
) -> Result<Handle<SshClient>, String> {
    let config = Arc::new(client::Config::default());
    let mut handle = client::connect(config, (profile.host.as_str(), profile.port), SshClient)
        .await
        .map_err(|e| format!("SSH 连接 {}:{} 失败: {e}", profile.host, profile.port))?;

    let auth = match profile.auth_kind {
        AuthKind::Password => {
            let password = secret
                .ok_or_else(|| "缺少密码：未在 keyring 中找到该 profile 的口令".to_string())?;
            handle
                .authenticate_password(&profile.username, password)
                .await
                .map_err(|e| format!("密码认证失败: {e}"))?
        }
        AuthKind::Privatekey => {
            let key_path = profile
                .private_key_path
                .as_deref()
                .ok_or_else(|| "私钥认证缺少 privateKeyPath".to_string())?;
            // secret 为私钥口令（可能为空）。
            let key = load_secret_key(key_path, secret)
                .map_err(|e| format!("加载私钥 {key_path} 失败: {e}"))?;
            let key = PrivateKeyWithHashAlg::new(Arc::new(key), None);
            handle
                .authenticate_publickey(&profile.username, key)
                .await
                .map_err(|e| format!("私钥认证失败: {e}"))?
        }
    };

    if !auth.success() {
        return Err("SSH 认证被服务器拒绝".to_string());
    }
    Ok(handle)
}

/// 在已认证连接上执行单条命令，收集 stdout+stderr 与退出码。
async fn exec(handle: &Handle<SshClient>, command: &str) -> Result<ExecOutput, String> {
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("打开 SSH 会话通道失败: {e}"))?;
    channel
        .exec(true, command)
        .await
        .map_err(|e| format!("执行命令失败: {e}"))?;

    let mut buf: Vec<u8> = Vec::new();
    // 退出码：无显式 ExitStatus 时按 -1 处理（视为失败）。
    let mut exit_code: i32 = -1;

    while let Some(msg) = channel.wait().await {
        match msg {
            ChannelMsg::Data { data } => buf.extend_from_slice(&data),
            ChannelMsg::ExtendedData { data, .. } => buf.extend_from_slice(&data),
            ChannelMsg::ExitStatus { exit_status } => exit_code = exit_status as i32,
            ChannelMsg::Eof | ChannelMsg::Close => {}
            _ => {}
        }
    }

    Ok(ExecOutput {
        exit_code,
        output: String::from_utf8_lossy(&buf).into_owned(),
    })
}

/// 格式化单步日志：命令 + 退出码 + 末尾输出（截断过长输出）。
fn format_step(title: &str, cmd: &str, out: &ExecOutput) -> String {
    let trimmed = out.output.trim_end();
    // 仅保留末尾 ~2000 字符，避免 apt 海量输出淹没日志。
    let tail = if trimmed.len() > 2000 {
        let start = trimmed.len() - 2000;
        // 对齐到字符边界，避免切碎 UTF-8。
        let start = (start..trimmed.len())
            .find(|i| trimmed.is_char_boundary(*i))
            .unwrap_or(trimmed.len());
        format!("...(已截断)\n{}", &trimmed[start..])
    } else {
        trimmed.to_string()
    };
    let status = if out.exit_code == 0 {
        "成功".to_string()
    } else {
        format!("失败(exit={})", out.exit_code)
    };
    format!(
        "===== {title} [{status}] =====\n$ {cmd}\n{tail}\n",
        title = title,
        status = status,
        cmd = cmd,
        tail = tail,
    )
}
