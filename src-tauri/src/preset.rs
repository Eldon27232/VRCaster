//! 参数预设：常用编码参数组合，存 data_dir()/presets.json。

use crate::types::Preset;
use std::fs;
use std::path::PathBuf;

fn presets_path() -> PathBuf {
    crate::config::data_dir().join("presets.json")
}

fn read() -> Result<Vec<Preset>, String> {
    let p = presets_path();
    if !p.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(&p).map_err(|e| format!("读取预设失败: {e}"))?;
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("解析预设失败: {e}"))
}

fn write(items: &[Preset]) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(items).map_err(|e| format!("序列化预设失败: {e}"))?;
    fs::write(presets_path(), json).map_err(|e| format!("写入预设失败: {e}"))
}

#[tauri::command]
pub fn list_presets() -> Result<Vec<Preset>, String> {
    read()
}

/// 保存预设：同 id 覆盖，否则追加。
#[tauri::command]
pub fn save_preset(preset: Preset) -> Result<(), String> {
    let mut items = read()?;
    if let Some(slot) = items.iter_mut().find(|p| p.id == preset.id) {
        *slot = preset;
    } else {
        items.push(preset);
    }
    write(&items)
}

#[tauri::command]
pub fn delete_preset(id: String) -> Result<(), String> {
    let mut items = read()?;
    items.retain(|p| p.id != id);
    write(&items)
}
