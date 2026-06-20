//! 压制历史记录：成品文件名 / URL / 大小 / 时间，存 data_dir()/history.json。

use crate::types::HistoryEntry;
use std::fs;
use std::path::PathBuf;

/// 历史上限，超出截断（最新在前）。
const MAX_ENTRIES: usize = 500;

fn history_path() -> PathBuf {
    crate::config::data_dir().join("history.json")
}

fn read() -> Result<Vec<HistoryEntry>, String> {
    let p = history_path();
    if !p.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(&p).map_err(|e| format!("读取历史失败: {e}"))?;
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("解析历史失败: {e}"))
}

fn write(items: &[HistoryEntry]) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(items).map_err(|e| format!("序列化历史失败: {e}"))?;
    fs::write(history_path(), json).map_err(|e| format!("写入历史失败: {e}"))
}

#[tauri::command]
pub fn list_history() -> Result<Vec<HistoryEntry>, String> {
    read()
}

/// 追加一条历史（同 id 去重；最新在前；超上限截断）。
#[tauri::command]
pub fn add_history(entry: HistoryEntry) -> Result<(), String> {
    let mut items = read()?;
    items.retain(|e| e.id != entry.id);
    items.insert(0, entry);
    if items.len() > MAX_ENTRIES {
        items.truncate(MAX_ENTRIES);
    }
    write(&items)
}

#[tauri::command]
pub fn delete_history(id: String) -> Result<(), String> {
    let mut items = read()?;
    items.retain(|e| e.id != id);
    write(&items)
}

#[tauri::command]
pub fn clear_history() -> Result<(), String> {
    write(&[])
}
