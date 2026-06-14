//! 队列调度：三种并发模式（流水线 / 同步处理+顺序上传 / 同步处理+同步上传）。
//! 契约见 docs/contract.md §4、design.md §5。

use crate::types::{LogEvent, QueueItem, QueueMode};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

/// 向前端发一条日志事件（失败不影响调度）。
fn log(app: &AppHandle, item_id: Option<&str>, level: &str, message: impl Into<String>) {
    let _ = app.emit(
        "log",
        LogEvent {
            item_id: item_id.map(|s| s.to_string()),
            level: level.into(),
            message: message.into(),
        },
    );
}

/// 压制单个 item，成功返回成品路径。
/// 复用 encode 模块的命令实现（薄封装互调）。
async fn encode_one(
    app: AppHandle,
    item: QueueItem,
) -> Result<String, String> {
    log(&app, Some(&item.id), "info", "开始压制");
    match crate::encode::start_encode(app.clone(), item.id.clone(), item.media, item.params).await {
        Ok(out) => {
            log(&app, Some(&item.id), "info", format!("压制完成: {out}"));
            Ok(out)
        }
        Err(e) => {
            log(&app, Some(&item.id), "error", format!("压制失败: {e}"));
            Err(e)
        }
    }
}

/// 上传单个 item 的成品，成功返回最终 URL。
/// 复用 upload 模块的命令实现（内部按 profile_id 自取 profile 与 keyring 凭据）。
async fn upload_one(
    app: AppHandle,
    item_id: String,
    out_path: String,
    profile_id: String,
) -> Result<String, String> {
    log(&app, Some(&item_id), "info", "开始上传");
    match crate::upload::start_upload(app.clone(), item_id.clone(), out_path, profile_id).await {
        Ok(url) => {
            log(&app, Some(&item_id), "info", format!("上传完成: {url}"));
            Ok(url)
        }
        Err(e) => {
            log(&app, Some(&item_id), "error", format!("上传失败: {e}"));
            Err(e)
        }
    }
}

/// 按模式调度队列。进度与状态通过事件上报。
/// 单个 item 失败仅记录日志，不中断整个队列。
#[tauri::command]
pub async fn run_queue(
    app: AppHandle,
    items: Vec<QueueItem>,
    mode: QueueMode,
    profile_id: String,
) -> Result<(), String> {
    // 预热 ffmpeg（幂等缓存）；失败直接整队返回，因为无 ffmpeg 任何压制都没意义。
    let _ff = crate::ffmpeg::ensure_ffmpeg(app.clone()).await?;

    if items.is_empty() {
        return Ok(());
    }

    match mode {
        QueueMode::Pipeline => run_pipeline(app, items, profile_id).await,
        QueueMode::ParallelEncodeSerialUpload => {
            run_parallel_encode_serial_upload(app, items, profile_id).await
        }
        QueueMode::ParallelAll => run_parallel_all(app, items, profile_id).await,
    }
}

/// 流水线：压制任务串行处理 items 依次 start_encode，每完成一个就把 (id, out) 送入 channel；
/// 上传任务从 channel 取并 start_upload —— 实现"压 N+1 与 传 N 重叠"。
async fn run_pipeline(
    app: AppHandle,
    items: Vec<QueueItem>,
    profile_id: String,
) -> Result<(), String> {
    let (tx, mut rx) = mpsc::channel::<(String, String)>(items.len().max(1));

    // 压制任务：串行压每个 item，成功的 (id, out) 投递给上传任务。
    let encode_app = app.clone();
    let encoder = tokio::spawn(async move {
        for item in items {
            let id = item.id.clone();
            if let Ok(out) = encode_one(encode_app.clone(), item).await {
                // 上传任务已退出（接收端关闭）则停止投递，但仍继续压完剩余以反馈日志。
                if tx.send((id, out)).await.is_err() {
                    break;
                }
            }
        }
        // tx drop → 上传任务的 recv 返回 None 退出循环。
    });

    // 上传任务：从 channel 串行取并上传，与下一个 item 的压制重叠。
    let upload_app = app.clone();
    let uploader = tokio::spawn(async move {
        while let Some((id, out)) = rx.recv().await {
            let _ = upload_one(upload_app.clone(), id, out, profile_id.clone()).await;
        }
    });

    let _ = encoder.await;
    let _ = uploader.await;
    Ok(())
}

/// 全部并发压制，全部完成后按原始顺序串行上传。
async fn run_parallel_encode_serial_upload(
    app: AppHandle,
    items: Vec<QueueItem>,
    profile_id: String,
) -> Result<(), String> {
    // 并发压制所有 item，保留各自 id 与原始顺序索引。
    let mut handles = Vec::with_capacity(items.len());
    for item in items {
        let app = app.clone();
        let id = item.id.clone();
        handles.push(tokio::spawn(async move {
            let res = encode_one(app, item).await;
            (id, res)
        }));
    }

    // 按 join 顺序（= 原始 spawn 顺序）收集成功的成品，保持序。
    let mut encoded: Vec<(String, String)> = Vec::with_capacity(handles.len());
    for h in handles {
        if let Ok((id, Ok(out))) = h.await {
            encoded.push((id, out));
        }
    }

    // 全部压制完成后，按序串行上传。
    for (id, out) in encoded {
        let _ = upload_one(app.clone(), id, out, profile_id.clone()).await;
    }
    Ok(())
}

/// 每个 item 一个 task 串起 encode → upload，全部并发。
async fn run_parallel_all(
    app: AppHandle,
    items: Vec<QueueItem>,
    profile_id: String,
) -> Result<(), String> {
    let mut handles = Vec::with_capacity(items.len());
    for item in items {
        let app = app.clone();
        let profile_id = profile_id.clone();
        let id = item.id.clone();
        handles.push(tokio::spawn(async move {
            // 单 item 内 encode → upload 串行；item 之间并发。
            if let Ok(out) = encode_one(app.clone(), item).await {
                let _ = upload_one(app, id, out, profile_id).await;
            }
        }));
    }

    // 等待全部 item 流水完成；单个 task 失败已在内部记录，不中断其余。
    for h in handles {
        let _ = h.await;
    }
    Ok(())
}
