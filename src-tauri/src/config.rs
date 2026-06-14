//! 服务器 profile 管理 + keyring 凭据存取 + 应用设置 + 配置导出/导入。
//! 契约见 docs/contract.md §4。密码/私钥口令存系统密钥环（service="vrcaster", account=profile.id）。

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::types::{AppSettings, ServerProfile};

/// keyring 服务名，account 用 profile.id。
const KEYRING_SERVICE: &str = "vrcaster";

// ---------------------------------------------------------------------------
// 内部接口（供其它模块调用，契约 §4 / 跨模块内部接口）
// ---------------------------------------------------------------------------

/// 应用配置目录：dirs::config_dir()/"vrcaster"，确保存在。
pub fn data_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vrcaster");
    // 幂等创建；失败时仍返回路径，由后续读写报错。
    let _ = fs::create_dir_all(&dir);
    dir
}

fn profiles_path() -> PathBuf {
    data_dir().join("profiles.json")
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

/// 读取全部 profile（文件不存在视为空）。
fn read_profiles() -> Result<Vec<ServerProfile>, String> {
    let path = profiles_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(&path).map_err(|e| format!("读取 profiles.json 失败: {e}"))?;
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("解析 profiles.json 失败: {e}"))
}

/// 原子化写入全部 profile（先写临时文件再 rename）。
fn write_profiles(profiles: &[ServerProfile]) -> Result<(), String> {
    let path = profiles_path();
    let json = serde_json::to_vec_pretty(profiles)
        .map_err(|e| format!("序列化 profiles 失败: {e}"))?;
    write_atomic(&path, &json)
}

/// 原子写：写到同目录 .tmp 后 rename 覆盖，避免半写损坏。
fn write_atomic(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("写入 {} 失败: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("替换 {} 失败: {e}", path.display()))?;
    Ok(())
}

/// 按 id 取单个 profile。
pub fn get_profile(id: &str) -> Result<ServerProfile, String> {
    read_profiles()?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("未找到 profile: {id}"))
}

/// 从 keyring 取密码/口令；不存在返回 Ok(None)。
pub fn get_secret(id: &str) -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, id).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(s) => Ok(Some(s)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// 读取 settings.json，缺失/为空返回默认值。
fn read_settings() -> Result<AppSettings, String> {
    let path = settings_path();
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let bytes = fs::read(&path).map_err(|e| format!("读取 settings.json 失败: {e}"))?;
    if bytes.is_empty() {
        return Ok(AppSettings::default());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("解析 settings.json 失败: {e}"))
}

fn write_settings(settings: &AppSettings) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(settings)
        .map_err(|e| format!("序列化 settings 失败: {e}"))?;
    write_atomic(&settings_path(), &json)
}

// ---------------------------------------------------------------------------
// 导出/导入载体（不含凭据）
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportBundle {
    profiles: Vec<ServerProfile>,
    settings: AppSettings,
}

// ---------------------------------------------------------------------------
// Tauri 命令（薄封装，均同步，返回 Result<_, String>）
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_profiles() -> Result<Vec<ServerProfile>, String> {
    read_profiles()
}

#[tauri::command]
pub fn save_profile(profile: ServerProfile, secret: Option<String>) -> Result<(), String> {
    let mut profiles = read_profiles()?;
    // 同 id 覆盖，否则追加。
    if let Some(slot) = profiles.iter_mut().find(|p| p.id == profile.id) {
        *slot = profile.clone();
    } else {
        profiles.push(profile.clone());
    }
    write_profiles(&profiles)?;

    // secret 有值才写 keyring（None 表示不改动现有凭据）。
    if let Some(s) = secret {
        let entry =
            keyring::Entry::new(KEYRING_SERVICE, &profile.id).map_err(|e| e.to_string())?;
        entry.set_password(&s).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_profile(id: String) -> Result<(), String> {
    let mut profiles = read_profiles()?;
    let before = profiles.len();
    profiles.retain(|p| p.id != id);
    if profiles.len() != before {
        write_profiles(&profiles)?;
    }

    // 清除 keyring 条目，不存在时忽略。
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &id) {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

#[tauri::command]
pub fn generate_access_key() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    // 32 位十六进制 = 16 字节随机。
    (0..16)
        .map(|_| format!("{:02x}", rng.random_range(0u16..256) as u8))
        .collect()
}

#[tauri::command]
pub fn get_settings() -> Result<AppSettings, String> {
    read_settings()
}

#[tauri::command]
pub fn set_settings(settings: AppSettings) -> Result<(), String> {
    write_settings(&settings)
}

#[tauri::command]
pub fn export_config(path: String) -> Result<(), String> {
    let bundle = ExportBundle {
        profiles: read_profiles()?,
        settings: read_settings()?,
    };
    let json = serde_json::to_vec_pretty(&bundle)
        .map_err(|e| format!("序列化导出内容失败: {e}"))?;
    fs::write(&path, &json).map_err(|e| format!("写入导出文件 {path} 失败: {e}"))
}

#[tauri::command]
pub fn import_config(path: String) -> Result<(), String> {
    let bytes = fs::read(&path).map_err(|e| format!("读取导入文件 {path} 失败: {e}"))?;
    let bundle: ExportBundle =
        serde_json::from_slice(&bytes).map_err(|e| format!("解析导入文件失败: {e}"))?;

    // 合并 profiles：同 id 覆盖，新 id 追加（不触碰 keyring，导入文件不含凭据）。
    let mut profiles = read_profiles()?;
    for incoming in bundle.profiles {
        if let Some(slot) = profiles.iter_mut().find(|p| p.id == incoming.id) {
            *slot = incoming;
        } else {
            profiles.push(incoming);
        }
    }
    write_profiles(&profiles)?;

    // settings 整体覆盖。
    write_settings(&bundle.settings)?;
    Ok(())
}
